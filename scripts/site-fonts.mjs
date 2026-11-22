import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile } from "node:fs/promises";
import path from "node:path";
import { EMOJI_FONT_FILE, EMOJI_FONT_INTEGRITY, EMOJI_FONT_SIZE } from "../crates/todo-browser/wwwroot/fonts.js";

const source = new URL("../crates/todo-controls/assets/fonts/", import.meta.url);

// Stage canonical licensed assets before prepareSite hashes the immutable tree and offline cache.
export async function copySiteFonts(root) {
  const bytes = await readFile(new URL(EMOJI_FONT_FILE, source));
  const integrity = `sha256-${createHash("sha256").update(bytes).digest("base64")}`;
  if (bytes.length !== EMOJI_FONT_SIZE || integrity !== EMOJI_FONT_INTEGRITY) {
    throw new Error("The bundled Noto emoji font does not match its pinned size and integrity.");
  }
  const destination = path.join(root, "fonts");
  await mkdir(destination, { recursive: true });
  await Promise.all([EMOJI_FONT_FILE, "OFL.txt"].map(name => copyFile(new URL(name, source), path.join(destination, name))));
}
