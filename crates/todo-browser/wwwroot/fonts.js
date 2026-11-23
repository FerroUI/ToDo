// Complete licensed faces. Separate assets keep their bytes out of WebAssembly.
export const EMOJI_FONT_FILE = "Noto-COLRv1.ttf";
export const EMOJI_FONT_SIZE = 5_038_236;
export const EMOJI_FONT_INTEGRITY = "sha256-uOJepo24L55NCu6SH0QgvivjmIe9XIk6KtmHEFMfnQw=";
export const CJK_FONT_FILE = "WenQuanYiMicroHei.ttf";
export const CJK_FONT_SIZE = 4_626_872;
export const CJK_FONT_INTEGRITY = "sha256-PDkGACn9ETkf5UV3LdCuQCp5tG/ldKIpRVyfjAmCvmo=";

export function loadEmojiFont(fetcher = globalThis.fetch) {
  return loadFont(fetcher, EMOJI_FONT_FILE, EMOJI_FONT_SIZE, EMOJI_FONT_INTEGRITY, "emoji");
}

export function loadCjkFont(fetcher = globalThis.fetch) {
  return loadFont(fetcher, CJK_FONT_FILE, CJK_FONT_SIZE, CJK_FONT_INTEGRITY, "CJK");
}

async function loadFont(fetcher, file, size, integrity, label) {
  const response = await fetcher(new URL(`./fonts/${file}`, import.meta.url), {
    credentials: "same-origin",
    integrity,
  });
  if (!response.ok) throw new Error(`The ${label} font could not be downloaded (${response.status}). Reload to try again.`);
  const reader = response.body?.getReader();
  if (!reader) throw new Error(`The ${label} font is incomplete. Reload to try again.`);
  // Decode at most the pinned size. Do not trust Content-Length: HTTP compression
  // may legitimately change it, and a broken response must not grow memory forever.
  const bytes = new Uint8Array(size);
  let length = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      if (value.byteLength > size - length) {
        await reader.cancel().catch(() => {});
        throw new Error(`The ${label} font exceeds its expected size. Reload to try again.`);
      }
      bytes.set(value, length);
      length += value.byteLength;
    }
    if (length !== size) throw new Error(`The ${label} font is incomplete. Reload to try again.`);
    return bytes;
  } finally {
    reader.releaseLock();
  }
}
