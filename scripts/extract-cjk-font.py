#!/usr/bin/env python3
"""Reproduce the complete proportional face from the official Micro Hei release.

Maintenance only: pip install fonttools==4.61.1, then run this script with the
unmodified wqy-microhei-0.2.0-beta.tar.gz downloaded from the source in the font
README. Normal builds use the committed font and never need Python/fontTools.
No glyph, character-map, hinting, kerning or layout subsetting is performed.
"""

import hashlib
import io
from pathlib import Path
import sys
import tarfile

import fontTools
from fontTools.ttLib import TTFont


ARCHIVE_SHA256 = "2802ac8023aa36a66ea6e7445854e3a078d377ffff42169341bd237871f7213e"
FONT_SHA256 = "272435f569f3e7ffa7cea299d0b175346738b5922eccaedb168eadb63f8123da"


def main():
    if len(sys.argv) != 2:
        raise SystemExit("Usage: extract-cjk-font.py wqy-microhei-0.2.0-beta.tar.gz")
    if fontTools.__version__ != "4.61.1":
        raise SystemExit("Reproduction requires fonttools==4.61.1")
    source = Path(sys.argv[1]).read_bytes()
    if hashlib.sha256(source).hexdigest() != ARCHIVE_SHA256:
        raise SystemExit("The archive does not match the official pinned release")
    destination = Path(__file__).resolve().parents[1] / "crates/todo-controls/assets/fonts"
    with tarfile.open(fileobj=io.BytesIO(source)) as archive:
        # Read named members without extracting archive paths to the filesystem.
        font = TTFont(io.BytesIO(archive.extractfile("wqy-microhei/wqy-microhei.ttc").read()),
                      fontNumber=0, recalcTimestamp=False)
        output = io.BytesIO()
        font.save(output)
        data = output.getvalue()
        if len(data) != 4_626_376 or hashlib.sha256(data).hexdigest() != FONT_SHA256:
            raise SystemExit("The extracted complete face differs from the committed font")
        (destination / "WenQuanYiMicroHei.ttf").write_bytes(data)
        for original, name in [
            ("LICENSE_Apache2.txt", "WenQuanYiMicroHei-LICENSE.txt"),
            ("AUTHORS.txt", "WenQuanYiMicroHei-AUTHORS.txt"),
            ("README.txt", "WenQuanYiMicroHei-README.txt"),
        ]:
            (destination / name).write_bytes(archive.extractfile("wqy-microhei/" + original).read())
    print(f"Complete face: {len(data)} bytes; SHA-256 {FONT_SHA256}")


if __name__ == "__main__":
    main()
