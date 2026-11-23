# Bundled fallback fonts

Inter remains the primary UI face. The complete Noto emoji and Micro Hei text
faces below provide glyph fallback through FerroUI's font manager; text editing,
shaping, selection and IME still use the native controls. Color emoji precedes
the CJK text face where their symbol coverage overlaps.

## Noto Color Emoji

`Noto-COLRv1.ttf` is the complete color vector font from the official
[googlefonts/noto-emoji](https://github.com/googlefonts/noto-emoji) repository.
It includes flags, emoji modifiers, and supported ZWJ sequences; it is not a
sample-specific subset.

- Upstream commit: `99701e07d3d1fe1bdbcdb14964547f0fd592966f`
- Source: `2D/fonts/Noto-COLRv1.ttf`
- Version: `2.057`
- Size: `5,038,236` bytes
- SHA-256: `b8e25ea68db82f9e4d0aee921f4420be2be39887bd5c893a2ad98710531f9d0c`
- Format: OpenType COLRv1 with GSUB sequence shaping
- Font copyright: Copyright 2022 Google Inc.
- License: SIL Open Font License 1.1; the unmodified upstream license is [OFL.txt](OFL.txt)

The browser downloads the font as a separate, integrity-pinned asset in parallel
with its runtime. `scripts/prepare-site.mjs` copies this canonical font and its
license into the versioned site before generating offline-cache integrity data.
The WebAssembly binary does not embed the font. The desktop entry point embeds
the same file so that it also works on machines without an emoji system font.

The shared FerroUI font collection accepts bytes and owns no networking or host
paths. Updating the font requires updating the source revision, size, hash, and
browser integrity constant together.

## WenQuanYi Micro Hei

`WenQuanYiMicroHei.ttf` is the **complete proportional face** of the official
[WenQuanYi Micro Hei 0.2.0-beta release](https://sourceforge.net/projects/wqy/files/wqy-microhei/0.2.0-beta/).
It is extracted from face 0 of `wqy-microhei.ttc`; no glyph, character-map,
hinting, kerning, or layout subsetting is performed. The unused monospace face
is not bundled. This is an openly licensed fallback, not Microsoft typography.

- Archive: [wqy-microhei-0.2.0-beta.tar.gz](https://downloads.sourceforge.net/project/wqy/wqy-microhei/0.2.0-beta/wqy-microhei-0.2.0-beta.tar.gz)
- Archive size: `2,418,536` bytes
- Archive SHA-256: `2802ac8023aa36a66ea6e7445854e3a078d377ffff42169341bd237871f7213e`
- Original TTC size: `5,177,387` bytes
- Original TTC SHA-256: `e4bca8df123ce01b104780f576ea1a58b9a5ff1662a91124b6d3180cb6c88212`
- Bundled face size: `4,626,376` bytes
- Bundled face SHA-256: `272435f569f3e7ffa7cea299d0b175346738b5922eccaedb168eadb63f8123da`
- Format: OpenType TrueType, 49,531 glyphs and 34,600 Unicode character mappings
- License chosen for this distribution: **Apache License 2.0**, offered by the
  upstream dual Apache 2.0/GPLv3 release
- Copyright: digitized data © 2007 Google Corporation; © 2008–2009 WenQuanYi
  Project Board of Trustees. The upstream authors file and release notes retain
  contributor credits and the Ume-Font punctuation attribution.

The unmodified upstream [Apache license](WenQuanYiMicroHei-LICENSE.txt),
[authors/credits](WenQuanYiMicroHei-AUTHORS.txt), and
[release notes](WenQuanYiMicroHei-README.txt) accompany both browser assets and
native packages. The latter two preserve the source's legacy text encoding.
The application's MIT license does not replace the font license.

The character map covers all Han `U+4E00–U+9FC3` documented by this release,
all 11,172 precomposed Hangul syllables `U+AC00–U+D7A3`, common Hiragana and
Katakana, and fullwidth Latin/punctuation. It is a complete release face, **not
a claim of all current Unicode CJK coverage**: newer Han extension blocks,
some rare Kana, and some historic characters are absent. A later font upgrade
must verify the desired script coverage and license as well as changing the pin.

Reproduce the binary and notices with Python and `fonttools==4.61.1`:

```sh
python scripts/extract-cjk-font.py /path/to/wqy-microhei-0.2.0-beta.tar.gz
```

The script checks the archive SHA-256, extracts the first complete face with
`recalcTimestamp=False`, and verifies the resulting size/hash before writing.
Normal builds consume the committed assets and need no fontTools dependency.

Both fonts load in parallel with the browser runtime through same-origin,
integrity-pinned requests. The loader bounds decoded bytes by each exact size,
and the versioned service worker caches the same font and notice bytes for
offline use. Neither file is embedded in WebAssembly. Native hosts embed the
same two faces, so script coverage does not depend on installed system fonts.
Regression tests verify the full documented ranges from the actual `cmap`, the
native font fallback chain with no system fonts, and mixed Japanese, Chinese,
Korean and emoji text. The compiled browser rich-note review retains real IME
composition and its rendered editor screenshot.
