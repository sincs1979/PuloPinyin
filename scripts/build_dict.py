#!/usr/bin/env python3
"""Build data/system.tsv from rime-ice (雾凇拼音) character + base phrase tables."""

from __future__ import annotations

import argparse
import pathlib
import re
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]
VENDOR = ROOT / "data" / "vendor"
SOURCES = [
    (
        "8105.dict.yaml",
        "https://raw.githubusercontent.com/iDvel/rime-ice/main/cn_dicts/8105.dict.yaml",
        {"min_freq": 0, "max_len": 8, "keep_all": True},
    ),
    (
        "base.dict.yaml",
        "https://raw.githubusercontent.com/iDvel/rime-ice/main/cn_dicts/base.dict.yaml",
        {"min_freq": 50, "max_len": 6, "keep_all": False},
    ),
]

TRIPLE = re.compile(r"(.)\1\1")


def fetch(name: str, url: str, force: bool) -> pathlib.Path:
    VENDOR.mkdir(parents=True, exist_ok=True)
    dest = VENDOR / name
    if dest.exists() and dest.stat().st_size > 1000 and not force:
        print(f"  cache {dest}")
        return dest
    print(f"  downloading {url}")
    req = urllib.request.Request(url, headers={"User-Agent": "pulopinyin-dict/0.1"})
    with urllib.request.urlopen(req, timeout=60) as resp:
        data = resp.read()
    dest.write_bytes(data)
    print(f"  wrote {dest} ({len(data)} bytes)")
    return dest


def yaml_body(path: pathlib.Path) -> str:
    text = path.read_text(encoding="utf-8")
    marker = "\n...\n"
    if marker in text:
        return text.split(marker, 1)[1]
    # Windows / odd files
    if "\n...\r\n" in text:
        return text.split("\n...\r\n", 1)[1]
    return text


def normalize_pinyin(raw: str) -> str:
    out = []
    i = 0
    s = raw.strip().replace("u:", "v").replace("ü", "v").replace("Ü", "v")
    while i < len(s):
        c = s[i]
        if c in "ABCDEFGHIJKLMNOPQRSTUVWXYZ":
            out.append(c.lower())
        elif c in "abcdefghijklmnopqrstuvwxyz":
            out.append(c)
        elif c in " \t'’":
            if out and out[-1] != " ":
                out.append(" ")
        i += 1
    return "".join(out).strip()


def parse_yaml(path: pathlib.Path, min_freq: int, max_len: int, keep_all: bool) -> dict[tuple[str, str], int]:
    entries: dict[tuple[str, str], int] = {}
    skipped = 0
    for lineno, line in enumerate(yaml_body(path).splitlines(), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) < 2:
            skipped += 1
            continue
        word = parts[0].strip()
        pinyin = normalize_pinyin(parts[1])
        if not word or not pinyin:
            skipped += 1
            continue
        if not keep_all:
            if len(word) > max_len:
                skipped += 1
                continue
            if TRIPLE.search(word):
                skipped += 1
                continue
        freq = 1
        if len(parts) >= 3:
            try:
                freq = int(float(parts[2]))
            except ValueError:
                freq = 1
        if not keep_all and freq < min_freq:
            skipped += 1
            continue
        key = (word, pinyin)
        prev = entries.get(key, 0)
        if freq > prev:
            entries[key] = freq
    print(f"  {path.name}: kept {len(entries)} skipped {skipped}")
    return entries


def parse_tsv(path: pathlib.Path) -> dict[tuple[str, str], int]:
    entries: dict[tuple[str, str], int] = {}
    if not path.exists():
        return entries
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        parts = line.split("\t")
        if len(parts) < 2:
            continue
        word = parts[0].strip()
        pinyin = normalize_pinyin(parts[1])
        if not word or not pinyin:
            continue
        freq = int(parts[2]) if len(parts) >= 3 and parts[2].isdigit() else 1
        key = (word, pinyin)
        entries[key] = max(entries.get(key, 0), freq)
    return entries


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("-o", "--output", type=pathlib.Path, default=ROOT / "data" / "system.tsv")
    parser.add_argument("--refresh", action="store_true")
    args = parser.parse_args()

    print("==> fetching rime-ice dictionaries")
    merged: dict[tuple[str, str], int] = {}
    for name, url, opts in SOURCES:
        path = fetch(name, url, args.refresh)
        part = parse_yaml(path, **opts)
        for key, freq in part.items():
            merged[key] = max(merged.get(key, 0), freq)

    builtin = ROOT / "data" / "builtin.tsv"
    overlay = parse_tsv(builtin)
    print(f"  overlay builtin.tsv ({len(overlay)} entries)")
    for key, freq in overlay.items():
        merged[key] = max(merged.get(key, 0), freq)

    rows = sorted(merged.items(), key=lambda kv: (-kv[1], kv[0][1], kv[0][0]))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as f:
        f.write("# word\tpinyin\tfrequency\n")
        f.write("# generated from rime-ice 8105 + base; overlay data/builtin.tsv\n")
        for (word, pinyin), freq in rows:
            f.write(f"{word}\t{pinyin}\t{freq}\n")
    print(f"wrote {len(rows)} entries → {args.output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
