export const MAX_FILE_BYTES = 25 * 1024 * 1024;
export const MAX_FILES_BYTES = 128 * 1024 * 1024;
export const MAX_DOCUMENT_BYTES = 32 * 1024 * 1024;
export const MAX_ARCHIVE_BYTES = 256 * 1024 * 1024;
const ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

export function checkTextSize(text, limit, label) {
  if (typeof text !== "string" || text.length > limit || new Blob([text]).size > limit) {
    throw new Error(`${label} is too large or is not text.`);
  }
}

export function decodeFile(entry) {
  const { size, base64 } = entry;
  const padding = (3 - size % 3) % 3;
  if (!Number.isSafeInteger(size) || size < 0 || size > MAX_FILE_BYTES || typeof base64 !== "string"
    || base64.length !== Math.ceil(size / 3) * 4 || (padding && !base64.endsWith("=".repeat(padding)))) {
    throw new Error("A backup file has an invalid size or base64 representation.");
  }
  const body = base64.slice(0, base64.length - padding);
  if (/[^A-Za-z0-9+/]/.test(body) || (padding === 2 && (ALPHABET.indexOf(body.at(-1)) & 15))
    || (padding === 1 && (ALPHABET.indexOf(body.at(-1)) & 3))) {
    throw new Error("A backup file contains invalid or noncanonical base64 data.");
  }
  const binary = atob(base64);
  if (binary.length !== size) throw new Error("A backup file is incomplete.");
  const bytes = new Uint8Array(size);
  for (let index = 0; index < size; index++) bytes[index] = binary.charCodeAt(index);
  return bytes;
}

export function encodeFile(bytes) {
  const parts = [];
  for (let index = 0; index < bytes.length; index += 16_384) parts.push(String.fromCharCode(...bytes.subarray(index, index + 16_384)));
  return btoa(parts.join(""));
}

export async function digest(bytes) {
  return [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))]
    .map(value => value.toString(16).padStart(2, "0")).join("");
}

export function exactFields(value, fields, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)
    || Object.keys(value).length !== fields.length || fields.some(field => !Object.hasOwn(value, field))) {
    throw new Error(`${label} has unsupported or missing fields.`);
  }
}
