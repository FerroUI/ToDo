// Pinned complete COLRv1 font. A separate asset avoids adding its bytes to the WebAssembly module.
export const EMOJI_FONT_FILE = "Noto-COLRv1.ttf";
export const EMOJI_FONT_SIZE = 5_038_236;
export const EMOJI_FONT_INTEGRITY = "sha256-uOJepo24L55NCu6SH0QgvivjmIe9XIk6KtmHEFMfnQw=";

export async function loadEmojiFont(fetcher = globalThis.fetch) {
  const response = await fetcher(new URL(`./fonts/${EMOJI_FONT_FILE}`, import.meta.url), {
    credentials: "same-origin",
    integrity: EMOJI_FONT_INTEGRITY,
  });
  if (!response.ok) throw new Error(`The emoji font could not be downloaded (${response.status}). Reload to try again.`);
  const bytes = new Uint8Array(await response.arrayBuffer());
  if (bytes.byteLength !== EMOJI_FONT_SIZE) throw new Error("The emoji font is incomplete. Reload to try again.");
  return bytes;
}
