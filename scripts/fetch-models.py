#!/usr/bin/env python3
"""Download every model in src-tauri/models.json into <out>/<id>/ and
verify size and SHA-256. Existing verified files are kept.

Usage: fetch-models.py <models.json> <out_dir> [model_id ...]
"""
import hashlib
import json
import os
import sys
import urllib.request


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


manifest = json.load(open(sys.argv[1]))
out = sys.argv[2]
wanted = sys.argv[3:] or list(manifest["models"])
for model_id in wanted:
    model = manifest["models"][model_id]
    d = os.path.join(out, model_id)
    os.makedirs(d, exist_ok=True)
    for f in model["files"]:
        path = os.path.join(d, f["name"])
        if os.path.exists(path) and os.path.getsize(path) == f["size"] and sha256(path) == f["sha256"]:
            continue
        print(f"downloading {model_id}/{f['name']} ({f['size'] / 1e6:.1f} MB)", flush=True)
        urllib.request.urlretrieve(f["url"], path + ".part")
        if os.path.getsize(path + ".part") != f["size"] or sha256(path + ".part") != f["sha256"]:
            sys.exit(f"checksum mismatch for {model_id}/{f['name']}")
        os.replace(path + ".part", path)
    print(f"{model_id}: ok")
