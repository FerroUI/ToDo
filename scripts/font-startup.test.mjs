import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { EMOJI_FONT_FILE, EMOJI_FONT_INTEGRITY, EMOJI_FONT_SIZE, loadEmojiFont } from "../crates/todo-browser/wwwroot/fonts.js";
import { copySiteFonts } from "./site-fonts.mjs";

const font = await readFile(new URL("../crates/todo-controls/assets/fonts/Noto-COLRv1.ttf", import.meta.url));

test("the canonical complete emoji font matches the startup pin and has color and shaping tables", () => {
  assert.equal(font.length, EMOJI_FONT_SIZE);
  assert.equal(`sha256-${createHash("sha256").update(font).digest("base64")}`, EMOJI_FONT_INTEGRITY);
  assert.equal(font.readUInt32BE(0), 0x00010000, "an OpenType TrueType font");
  const tables = new Map(Array.from({ length: font.readUInt16BE(4) }, (_, index) => {
    const offset = 12 + index * 16;
    return [font.toString("ascii", offset, offset + 4), font.readUInt32BE(offset + 8)];
  }));
  for (const name of ["COLR", "CPAL", "GSUB", "cmap"]) assert(tables.has(name), `font includes ${name}`);
  assert.equal(font.readUInt16BE(tables.get("COLR")), 1, "compact COLRv1 color outlines");
});

test("browser startup requests a relative, integrity-pinned font and returns its bytes to the runtime", async () => {
  let requested = false;
  const bytes = await loadEmojiFont(async (url, options) => {
    requested = true;
    assert(url.pathname.endsWith(`/fonts/${EMOJI_FONT_FILE}`));
    assert.deepEqual(options, { credentials: "same-origin", integrity: EMOJI_FONT_INTEGRITY });
    return new Response(font);
  });
  assert(requested);
  assert(bytes instanceof Uint8Array);
  assert.deepEqual(Buffer.from(bytes), font);
});

test("a failed or truncated emoji download fails startup with an actionable error", async () => {
  await assert.rejects(loadEmojiFont(async () => new Response("missing", { status: 404 })), /could not be downloaded \(404\)/);
  await assert.rejects(loadEmojiFont(async () => new Response(font.subarray(0, 24))), /font is incomplete/);
});

test("site staging copies the canonical font and unmodified OFL notice before revision hashing", async t => {
  const root = await mkdtemp(path.join(tmpdir(), "todo-fonts-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  await copySiteFonts(root);
  assert.deepEqual(await readFile(path.join(root, "fonts", EMOJI_FONT_FILE)), font);
  const license = await readFile(path.join(root, "fonts", "OFL.txt"), "utf8");
  assert(license.includes("SIL OPEN FONT LICENSE Version 1.1"));
  assert.equal(license, await readFile(new URL("../crates/todo-controls/assets/fonts/OFL.txt", import.meta.url), "utf8"));
});
