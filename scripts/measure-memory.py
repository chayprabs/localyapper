#!/usr/bin/env python3
"""Measure LocalYapper's private memory in the states of plan P6.6.

Usage: measure-memory.py APP_BINARY MODEL_DIR OUT_MARKDOWN

APP_BINARY is a release build with the `test-hooks` feature (it drives the
app over the local test channel) and `tauri/custom-protocol`. MODEL_DIR
holds the parakeet-110m files (scripts/fetch-models.py checked their
SHA-256). The script prints a Markdown section and writes it to
OUT_MARKDOWN.

States: idle before any dictation; model loaded after one dictation of a
fixture clip; settings window open; idle after the model was evicted;
the same with the overlay window destroyed (plan P6.3: what keeping the
overlay alive costs). It also samples idle CPU for P6.4.

Private memory is USS where the OS exposes it (Linux, macOS) and
private bytes on Windows. WebView helpers are the app's child processes;
on macOS WebKit helpers are XPC services started by launchd, so the
script takes the WebKit processes that appeared after the app started.
"""

import json
import os
import platform
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import wave
from pathlib import Path

import psutil

ROOT = Path(__file__).resolve().parent.parent
MODEL_ID = "parakeet-110m"
FIXTURE = ROOT / "src-tauri" / "tests" / "fixtures" / "clip01_16k.wav"


def free_port():
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    port = s.getsockname()[1]
    s.close()
    return port


class Channel:
    def __init__(self, port, proc, log_path, timeout=60):
        start = time.time()
        while True:
            try:
                self.sock = socket.create_connection(("127.0.0.1", port), timeout=30)
                break
            except OSError:
                if proc.poll() is not None:
                    raise SystemExit("the app exited:\n" + Path(log_path).read_text(errors="replace"))
                if time.time() - start > timeout:
                    raise
                time.sleep(0.2)
        self.file = self.sock.makefile("rw", encoding="utf-8", newline="\n")

    def cmd(self, c):
        self.file.write(c + "\n")
        self.file.flush()
        return json.loads(self.file.readline())

    def state(self):
        return self.cmd("state")["state"]


def prepare_data_dir(model_dir):
    data = Path(tempfile.mkdtemp(prefix="localyapper-mem-"))
    manifest = json.loads((ROOT / "src-tauri" / "models.json").read_text())
    entry = manifest["models"][MODEL_ID]
    dst = data / "models" / MODEL_ID
    dst.mkdir(parents=True)
    for f in Path(model_dir).iterdir():
        if f.is_file():
            shutil.copy2(f, dst / f.name)
    marker = "".join(f"{x['name']} {x['size']} {x['sha256']}\n" for x in entry["files"])
    (dst / ".verified").write_text(marker, newline="\n")
    return data


def private_bytes(p):
    try:
        if os.name == "nt":
            return p.memory_info().private
        return p.memory_full_info().uss
    except (psutil.AccessDenied, psutil.NoSuchProcess, AttributeError):
        try:
            return p.memory_info().rss
        except psutil.Error:
            return 0


def helpers(app, before):
    """The app's WebView helper processes."""
    found = {}
    try:
        for c in app.children(recursive=True):
            found[c.pid] = c
    except psutil.Error:
        pass
    if platform.system() == "Darwin":
        for p in psutil.process_iter(["name", "create_time"]):
            name = p.info.get("name") or ""
            if "WebKit" in name and p.pid not in before:
                found[p.pid] = p
    return list(found.values())


def snapshot(app, before):
    rows = [("main", app.name(), private_bytes(app))]
    for h in helpers(app, before):
        try:
            rows.append(("helper", h.name(), private_bytes(h)))
        except psutil.Error:
            pass
    return rows


def idle_cpu(app, before, seconds=10):
    procs = [app] + helpers(app, before)
    for p in procs:
        try:
            p.cpu_percent(None)
        except psutil.Error:
            pass
    time.sleep(seconds)
    total = 0.0
    for p in procs:
        try:
            total += p.cpu_percent(None)
        except psutil.Error:
            pass
    return total


def clip_ms(path):
    with wave.open(str(path)) as w:
        return int(1000 * w.getnframes() / w.getframerate())


def wait(pred, timeout, what):
    start = time.time()
    while not pred():
        if time.time() - start > timeout:
            raise SystemExit(f"timed out waiting for {what}")
        time.sleep(0.25)


def main():
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    app_bin, model_dir, out = sys.argv[1], sys.argv[2], sys.argv[3]
    data = prepare_data_dir(model_dir)
    port = free_port()
    env = dict(os.environ)
    env.update(
        {
            "LOCALYAPPER_DATA_DIR": str(data),
            "LOCALYAPPER_TEST_PORT": str(port),
            "LOCALYAPPER_FAKE_AUDIO": str(FIXTURE),
            "LOCALYAPPER_TEST_SETTINGS": (
                "setup_complete=true;setup_step=done;auto_start=false;"
                f"speech_model={MODEL_ID};idle_unload_seconds=30;activation_mode=hold"
            ),
            "RUST_LOG": "info",
        }
    )
    before = {p.pid for p in psutil.process_iter()}
    log = open(data / "app.log", "w")
    proc = subprocess.Popen([app_bin], env=env, stdout=log, stderr=subprocess.STDOUT)
    app = psutil.Process(proc.pid)
    results = []

    def record(name):
        rows = snapshot(app, before)
        total = sum(b for _, _, b in rows) / (1024 * 1024)
        print(f"measured: {name}: {total:.1f} MB", file=sys.stderr, flush=True)
        results.append((name, rows))

    try:
        ch = Channel(port, proc, data / "app.log")
        wait(lambda: ch.state() is not None, 30, "controller")
        time.sleep(8)
        record("Idle, model not loaded yet")

        ch.cmd("down")
        time.sleep(clip_ms(FIXTURE) / 1000 + 0.3)
        ch.cmd("up")
        wait(lambda: ch.state()["dictations"] >= 1, 120, "the dictation")
        time.sleep(2)
        loaded = ch.state()["model_loaded"]
        record(f"Model loaded ({loaded})")

        ch.cmd("open_settings")
        time.sleep(8)
        record("Settings window open")
        ch.cmd("close_settings")

        wait(lambda: ch.state()["model_loaded"] is None, 90, "the model to be evicted")
        time.sleep(5)
        record("Idle, model evicted")
        settings_alive = "main" in ch.cmd("windows")["windows"]
        cpu = idle_cpu(app, before)

        ch.cmd("destroy_overlay")
        time.sleep(5)
        record("Idle, model evicted, overlay destroyed")
    finally:
        proc.kill()
        proc.wait()
        log.close()

    mb = lambda b: b / (1024 * 1024)
    system = {"Windows": "Windows", "Darwin": "macOS", "Linux": "Linux"}.get(platform.system(), platform.system())
    lines = [f"### {system} ({platform.platform()})", ""]
    lines.append("| State | Main process MB | WebView helpers MB | Total MB |")
    lines.append("|---|---|---|---|")
    totals = {}
    for state, rows in results:
        main_b = sum(b for kind, _, b in rows if kind == "main")
        help_b = sum(b for kind, _, b in rows if kind == "helper")
        totals[state] = main_b + help_b
        lines.append(f"| {state} | {mb(main_b):.1f} | {mb(help_b):.1f} | {mb(main_b + help_b):.1f} |")
    lines.append("")
    lines.append("Per process:")
    lines.append("")
    for state, rows in results:
        parts = ", ".join(f"{name} {mb(b):.1f}" for _, name, b in rows)
        lines.append(f"- {state}: {parts}")
    keep = totals["Idle, model evicted"] - totals["Idle, model evicted, overlay destroyed"]
    idle_main = next(
        sum(b for kind, _, b in rows if kind == "main")
        for state, rows in results
        if state == "Idle, model evicted"
    )
    a7 = mb(idle_main) < 40 and not settings_alive
    lines.append("")
    lines.append(f"Overlay window kept alive costs {mb(keep):.1f} MB (P6.3 threshold: 30 MB).")
    lines.append(f"Idle CPU over 10 s with the model evicted: {cpu:.1f} %.")
    lines.append(
        f"A7 (idle, settings closed, model evicted: main process under 40 MB private, "
        f"no settings WebView): main process {mb(idle_main):.1f} MB, settings WebView "
        f"{'alive' if settings_alive else 'not alive'}: {'met' if a7 else 'not met'}."
    )
    lines.append("")
    text = "\n".join(lines)
    print(text)
    Path(out).write_text(text + "\n", encoding="utf-8")
    shutil.rmtree(data, ignore_errors=True)


if __name__ == "__main__":
    main()
