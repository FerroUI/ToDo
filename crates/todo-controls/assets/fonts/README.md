# Noto Color Emoji

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
