#!/usr/bin/env python3
"""Write src-tauri/models.json with pinned URLs, sizes and SHA-256 values.

Asks the Hugging Face API for the current commit of each candidate
repository, pins every file URL to that commit, and records size and
SHA-256. Run on a machine that can reach huggingface.co (CI does this in
.github/workflows/manifest.yml).

Usage: gen-model-manifest.py [out_path]
"""
import hashlib
import json
import sys
import time
import urllib.request

OUT = sys.argv[1] if len(sys.argv) > 1 else "src-tauri/models.json"

CANDIDATES = {
    "parakeet-110m": {
        "name": "Parakeet 110M",
        "family": "nemo_ctc",
        "languages": ["en"],
        "license": "CC-BY-4.0",
        # Try the int8 repository first, then the fp32 one the app used before.
        "repos": ["csukuangfj/sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000-int8",
                  "csukuangfj/sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000"],
        "want": ["model.int8.onnx", "model.onnx", "tokens.txt"],
    },
    # The fp32 build the app shipped with. Kept so the bench can compare it
    # with int8 and so existing installs are recognised (plan P4.6).
    "parakeet-110m-fp32": {
        "name": "Parakeet 110M (fp32, legacy)",
        "family": "nemo_ctc",
        "languages": ["en"],
        "license": "CC-BY-4.0",
        "legacy": True,
        "repos": ["csukuangfj/sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000"],
        "want": ["model.onnx", "tokens.txt"],
    },
    "parakeet-0.6b-v3": {
        "name": "Parakeet 0.6B v3",
        "family": "nemo_transducer",
        "languages": ["bg", "cs", "da", "de", "el", "en", "es", "et", "fi", "fr", "hr", "hu",
                      "it", "lt", "lv", "mt", "nl", "pl", "pt", "ro", "ru", "sk", "sl", "sv", "uk"],
        "license": "CC-BY-4.0",
        "repos": ["csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v3-int8"],
        "want": ["encoder.int8.onnx", "decoder.int8.onnx", "joiner.int8.onnx", "tokens.txt"],
    },
    "moonshine-tiny-en": {
        "name": "Moonshine Tiny",
        "family": "moonshine",
        "languages": ["en"],
        "license": "MIT",
        "repos": ["csukuangfj/sherpa-onnx-moonshine-tiny-en-int8"],
        "want": None,  # every top-level .onnx and tokens.txt
    },
}


def get(url):
    for attempt in range(4):
        try:
            with urllib.request.urlopen(url, timeout=60) as r:
                return json.load(r)
        except Exception:
            if attempt == 3:
                raise
            time.sleep(5 * (attempt + 1))


def sha256_url(url):
    h = hashlib.sha256()
    with urllib.request.urlopen(url, timeout=120) as r:
        for chunk in iter(lambda: r.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def search_int8(query):
    """Repositories found by the Hub search whose name says int8."""
    try:
        found = get(f"https://huggingface.co/api/models?search={query}&limit=50")
    except Exception as e:  # noqa: BLE001
        print(f"search {query} failed: {e}", file=sys.stderr)
        return []
    return sorted(m["id"] for m in found if "int8" in m["id"].lower())


# The int8 build of Parakeet 110M has no confirmed repository name, so ask
# the Hub search before falling back to the fp32 repository.
extra = [r for r in search_int8("parakeet_tdt_ctc_110m") + search_int8("parakeet-tdt-ctc-110m")
         if r not in CANDIDATES["parakeet-110m"]["repos"]]
CANDIDATES["parakeet-110m"]["repos"] = (
    CANDIDATES["parakeet-110m"]["repos"][:1] + extra + CANDIDATES["parakeet-110m"]["repos"][1:])

models = {}
for model_id, spec in CANDIDATES.items():
    for repo in spec["repos"]:
        try:
            sha = get(f"https://huggingface.co/api/models/{repo}")["sha"]
            tree = get(f"https://huggingface.co/api/models/{repo}/tree/{sha}")
        except Exception as e:  # noqa: BLE001 - try the next repository
            print(f"skip {repo}: {e}", file=sys.stderr)
            continue
        files = []
        for f in tree:
            name = f["path"]
            if f.get("type") != "file":
                continue
            wanted = (name in spec["want"]) if spec["want"] else (
                name.endswith(".onnx") or name == "tokens.txt")
            if not wanted:
                continue
            lfs = f.get("lfs")
            files.append({
                "name": name,
                "url": f"https://huggingface.co/{repo}/resolve/{sha}/{name}",
                "size": (lfs or f)["size"],
                # LFS files report their SHA-256 as `oid`. Small files do not:
                # they are downloaded and hashed below.
                "sha256": lfs["oid"] if lfs else None,
            })
        # A repository with both builds keeps only the int8 one.
        names = {f["name"] for f in files}
        if "model.int8.onnx" in names and "model.onnx" in names and not spec.get("legacy"):
            files = [f for f in files if f["name"] != "model.onnx"]
        if not any(f["name"].endswith(".onnx") for f in files):
            print(f"skip {repo}: no model files", file=sys.stderr)
            continue
        entry = {k: spec[k] for k in ("name", "family", "languages", "license")}
        if spec.get("legacy"):
            entry["legacy"] = True
        entry.update({"repo": repo, "revision": sha,
                      "files": sorted(files, key=lambda f: f["name"])})
        models[model_id] = entry
        break
    else:
        print(f"WARNING: no repository found for {model_id}", file=sys.stderr)

# Without an int8 build the default already is the fp32 build: no
# separate legacy entry is needed.
if models.get("parakeet-110m", {}).get("repo") == models.get("parakeet-110m-fp32", {}).get("repo"):
    models.pop("parakeet-110m-fp32", None)
    print("no int8 build of parakeet-110m found; keeping fp32", file=sys.stderr)

for m in models.values():
    for f in m["files"]:
        if f["sha256"] is None:
            f["sha256"] = sha256_url(f["url"])

with open(OUT, "w", encoding="utf-8") as fh:
    json.dump({"schema": 1, "models": models}, fh, indent=2)
    fh.write("\n")
print(json.dumps({k: {"repo": v["repo"], "mb": round(sum(f["size"] for f in v["files"]) / 1e6, 1)}
                  for k, v in models.items()}, indent=2))
