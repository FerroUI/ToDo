#!/usr/bin/env python3
"""Reproduce the complete proportional face from the official Micro Hei release.

Maintenance only: pip install fonttools==4.61.1, then run this script with the
unmodified wqy-microhei-0.2.0-beta.tar.gz downloaded from the source in the font
README. Normal builds use the committed font and never need Python/fontTools.
No glyph, character-map, hinting, kerning or layout subsetting is performed.
The pinned release's composite Hangul advances are materialized in hmtx for
OpenType shaping. Original outlines and side bearings remain byte-identical.
"""

import hashlib
import io
from pathlib import Path
import struct
import sys
import tarfile

import fontTools
from fontTools.ttLib import TTFont
from fontTools.ttLib.tables.DefaultTable import DefaultTable


ARCHIVE_SHA256 = "2802ac8023aa36a66ea6e7445854e3a078d377ffff42169341bd237871f7213e"
EXTRACTED_SHA256 = "272435f569f3e7ffa7cea299d0b175346738b5922eccaedb168eadb63f8123da"
FONT_SHA256 = "3c39060029fd11391fe545772dd0ae402a79b46fe574a229455c9f8c0982be6a"
FONT_SIZE = 4_626_872
NORMALIZED_SYLLABLES = 11_170
MODIFICATION_NOTICE = (
    "Modified for FerroUI ToDo: materialized the inherited horizontal advances "
    "of 11,170 Hangul composite glyphs in hmtx for OpenType shaping. "
    "Original glyph outlines, side bearings, character maps, hinting and layout "
    "tables are unchanged. See the accompanying font provenance and Apache 2.0 license."
)


def normalize_complete_face(extracted):
    """Normalize only the proven metrics of the exact pinned complete face.

    This is intentionally not a generic font repair tool. Each affected glyph
    must use one untransformed, untranslated simple component's metrics. Both
    parent and component have a zero horizontal phantom origin, so the original
    parent side bearing is correct even when it differs from the component's.
    """
    if hashlib.sha256(extracted).hexdigest() != EXTRACTED_SHA256:
        raise ValueError("The extracted face does not match the pinned source")
    face = TTFont(io.BytesIO(extracted), recalcBBoxes=False, recalcTimestamp=False)
    raw = {tag: face.reader[tag] for tag in face.reader.keys()}
    cmap, glyphs, metrics = face.getBestCmap(), face["glyf"], face["hmtx"].metrics
    units = face["head"].unitsPerEm
    if units != 2048:
        raise ValueError("Unexpected design units in the pinned face")
    hmtx = bytearray(raw["hmtx"])
    modified = 0
    for codepoint in range(0xAC00, 0xD7A4):
        name = cmap[codepoint]
        glyph = glyphs[name]
        advance, bearing = metrics[name]
        if advance == units:
            continue
        inherited = [c for c in glyph.components if c.flags & 0x0200] if glyph.isComposite() else []
        if advance != 256 or len(inherited) != 1:
            raise ValueError(f"Unexpected composite metrics for U+{codepoint:04X}")
        component = inherited[0]
        component_name, transform = component.getComponentInfo()
        child = glyphs[component_name]
        child_advance, child_bearing = metrics[component_name]
        if (
            transform != (1, 0, 0, 1, 0, 0)
            or child.isComposite()
            or child_advance != units
            or child.xMin - child_bearing != 0
            or glyph.xMin - bearing != 0
        ):
            raise ValueError(f"Unsupported inherited metrics for U+{codepoint:04X}")
        glyph_id = face.getGlyphID(name)
        if glyph_id >= face["hhea"].numberOfHMetrics:
            raise ValueError("A shared final advance cannot be patched independently")
        # Only the advance changes. Copying the child's side bearing would move
        # glyphs such as 국: its parent's bearing is 168, the child's is 368.
        struct.pack_into(">H", hmtx, glyph_id * 4, child_advance)
        modified += 1
    if modified != NORMALIZED_SYLLABLES:
        raise ValueError(f"Expected {NORMALIZED_SYLLABLES} normalized syllables, got {modified}")

    names = face["name"]
    for record in names.names:
        if record.nameID == 5:
            record.string = record.toUnicode() + "; FerroUI ToDo metrics 1"
    for platform, encoding, language in [(3, 1, 0x0409), (1, 0, 0)]:
        previous = names.getName(10, platform, encoding, language)
        description = (previous.toUnicode() + "\n" if previous else "") + MODIFICATION_NOTICE
        names.setName(description, 10, platform, encoding, language)

    # Rebuild the SFNT directory/checksums, preserving every untouched table's
    # exact bytes. Loading glyf above must never cause outline recompilation.
    normalized = TTFont(recalcBBoxes=False, recalcTimestamp=False)
    normalized.sfntVersion = face.sfntVersion
    for tag, data in raw.items():
        table = DefaultTable(tag)
        table.data = bytes(hmtx) if tag == "hmtx" else data
        normalized[tag] = table
    normalized["name"] = names
    output = io.BytesIO()
    normalized.save(output)
    data = output.getvalue()
    verified = TTFont(io.BytesIO(data), lazy=True)
    for tag, original in raw.items():
        if tag not in {"head", "hmtx", "name"} and verified.reader[tag] != original:
            raise ValueError(f"Normalization unexpectedly changed the {tag} table")
    return data


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
        data = normalize_complete_face(output.getvalue())
        if len(data) != FONT_SIZE or hashlib.sha256(data).hexdigest() != FONT_SHA256:
            raise SystemExit("The normalized complete face differs from the committed font")
        (destination / "WenQuanYiMicroHei.ttf").write_bytes(data)
        for original, name in [
            ("LICENSE_Apache2.txt", "WenQuanYiMicroHei-LICENSE.txt"),
            ("AUTHORS.txt", "WenQuanYiMicroHei-AUTHORS.txt"),
            ("README.txt", "WenQuanYiMicroHei-README.txt"),
        ]:
            (destination / name).write_bytes(archive.extractfile("wqy-microhei/" + original).read())
    print(f"Complete face: {len(data)} bytes; {NORMALIZED_SYLLABLES} normalized advances; SHA-256 {FONT_SHA256}")


if __name__ == "__main__":
    main()
