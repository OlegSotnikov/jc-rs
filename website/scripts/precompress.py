#!/usr/bin/env python3
"""Stage the site's static assets with .br and .gz variants beside each file.

    precompress.py SRC DST [SRC DST ...]

Copies each SRC tree into DST and writes NAME.br / NAME.gz next to every file
worth compressing. nginx's `brotli_static` / `gzip_static` (already on globally
on the host) serve those variants when the browser asks, so nothing is
compressed per request and nothing is buffered to a temp file.

Staged into a separate tree, and deliberately not written into .next/ itself:
the Docker image is built from .next/, and the image does not need the
compressed variants — the container never serves these paths once nginx takes
them over.

Compression is worth it for text and wasm; already-packed formats (woff2, png,
jpeg) only grow. A variant is kept only when it is actually smaller, and files
under 1 KiB are skipped to match the server's brotli_min_length.
"""

import gzip
import pathlib
import shutil
import sys

try:
    import brotli
except ImportError:
    sys.exit("precompress: the 'brotli' python module is required (pip install brotli)")

COMPRESSIBLE = {
    ".js", ".mjs", ".css", ".map", ".json", ".txt", ".xml", ".svg", ".html",
    ".wasm", ".ico", ".webmanifest",
}
MIN_SIZE = 1024


def stage(src: pathlib.Path, dst: pathlib.Path) -> tuple[int, int, int, int]:
    files = variants = raw_total = compressed_total = 0
    for f in sorted(src.rglob("*")):
        if not f.is_file() or f.suffix in (".br", ".gz"):
            continue
        rel = f.relative_to(src)
        out = dst / rel
        out.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(f, out)
        files += 1

        data = f.read_bytes()
        raw_total += len(data)
        if f.suffix not in COMPRESSIBLE or len(data) < MIN_SIZE:
            continue

        br = brotli.compress(data, quality=11)
        if len(br) < len(data):
            out.with_name(out.name + ".br").write_bytes(br)
            variants += 1
            compressed_total += len(br)

        # mtime=0 keeps the bytes identical across runs, so rsync sees an
        # unchanged deploy as unchanged.
        gz = gzip.compress(data, compresslevel=9, mtime=0)
        if len(gz) < len(data):
            out.with_name(out.name + ".gz").write_bytes(gz)
            variants += 1

    return files, variants, raw_total, compressed_total


def main() -> None:
    args = sys.argv[1:]
    if not args or len(args) % 2:
        sys.exit(__doc__.strip().splitlines()[2].strip())
    total_files = total_variants = 0
    for i in range(0, len(args), 2):
        src, dst = pathlib.Path(args[i]), pathlib.Path(args[i + 1])
        if not src.is_dir():
            sys.exit(f"precompress: {src} is not a directory")
        files, variants, raw, br = stage(src, dst)
        total_files += files
        total_variants += variants
        pct = f", br total {br / raw:.0%} of raw" if br else ""
        print(f"{src} -> {dst}: {files} files, {variants} variants{pct}")
    print(f"staged {total_files} files, {total_variants} compressed variants")


if __name__ == "__main__":
    main()
