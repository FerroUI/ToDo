import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile } from "node:fs/promises";
import path from "node:path";
import {
  CJK_FONT_FILE, CJK_FONT_INTEGRITY, CJK_FONT_SIZE,
  EMOJI_FONT_FILE, EMOJI_FONT_INTEGRITY, EMOJI_FONT_SIZE,
} from "../crates/todo-browser/wwwroot/fonts.js";

const source = new URL("../crates/todo-controls/assets/fonts/", import.meta.url);

// Stage canonical licensed assets before prepareSite hashes the immutable tree and offline cache.
export async function copySiteFonts(root) {
  await Promise.all([
    [EMOJI_FONT_FILE, EMOJI_FONT_SIZE, EMOJI_FONT_INTEGRITY],
    [CJK_FONT_FILE, CJK_FONT_SIZE, CJK_FONT_INTEGRITY],
  ].map(async ([file, size, expected]) => {
    const bytes = await readFile(new URL(file, source));
    const integrity = `sha256-${createHash("sha256").update(bytes).digest("base64")}`;
    if (bytes.length !== size || integrity !== expected) {
      throw new Error(`The bundled ${file} font does not match its pinned size and integrity.`);
    }
  }));
  const destination = path.join(root, "fonts");
  await mkdir(destination, { recursive: true });
  await Promise.all([
    EMOJI_FONT_FILE, "OFL.txt", CJK_FONT_FILE, "WenQuanYiMicroHei-LICENSE.txt",
    "WenQuanYiMicroHei-AUTHORS.txt", "WenQuanYiMicroHei-README.txt",
  ].map(name => copyFile(new URL(name, source), path.join(destination, name))));
}
