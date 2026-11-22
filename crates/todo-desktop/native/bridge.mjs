import { readSync, writeSync } from "node:fs";

export const MAX_FRAME = 272 * 1024 * 1024;
let nextId = 0;

function readExact(size, eof = false) {
  const buffer = Buffer.allocUnsafe(size);
  let offset = 0;
  while (offset < size) {
    const count = readSync(0, buffer, offset, size - offset);
    if (!count) {
      if (eof && offset === 0) return null;
      throw new Error("The native host connection closed during a message.");
    }
    offset += count;
  }
  return buffer;
}

export function receive() {
  const header = readExact(4, true);
  if (!header) return null;
  const size = header.readUInt32LE();
  if (!size || size > MAX_FRAME) throw new Error("The native host message exceeds its size limit.");
  return JSON.parse(readExact(size).toString("utf8"));
}

export function send(value) {
  const bytes = Buffer.from(JSON.stringify(value));
  if (!bytes.length || bytes.length > MAX_FRAME) throw new Error("The native service result exceeds its size limit.");
  const header = Buffer.allocUnsafe(4); header.writeUInt32LE(bytes.length);
  for (const buffer of [header, bytes]) {
    let offset = 0;
    while (offset < buffer.length) offset += writeSync(1, buffer, offset, buffer.length - offset);
  }
}

// This blocks only the isolated service process. Rust's UI thread processes the
// callback and returns its revision-checked result; network work never blocks UI.
export function ui(method, args = {}) {
  const id = ++nextId;
  send({ type: "callback", id, method, args });
  const result = receive();
  if (!result || result.type !== "callbackResult" || result.id !== id) throw new Error("The native host returned an invalid response.");
  if (result.error) throw new Error(result.error);
  return result.value;
}
