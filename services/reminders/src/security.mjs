import { createHash, timingSafeEqual } from "node:crypto";
import { lookup } from "node:dns";
import { Agent } from "node:https";
import { isIP } from "node:net";

export class HttpError extends Error {
  constructor(status, message, details = {}) { super(message); this.status = status; this.details = details; }
}
export const hash = value => createHash("sha256").update(value).digest("base64url");
export function equalSecret(value, digest) {
  return typeof value === "string" && typeof digest === "string"
    && timingSafeEqual(Buffer.from(hash(value)), Buffer.from(digest.padEnd(43, " ").slice(0, 43)));
}
export function token(value, length = 43) {
  return typeof value === "string" && value.length === length && /^[A-Za-z0-9_-]+$/.test(value)
    && Buffer.from(value, "base64url").toString("base64url") === value;
}
export function exactKeys(value, keys) {
  if (!value || typeof value !== "object" || Array.isArray(value) || Object.keys(value).some(key => !keys.includes(key))) {
    throw new HttpError(400, "Unexpected request fields.");
  }
}
export function subscription(value, config) {
  exactKeys(value, ["endpoint", "keys", "expirationTime"]);
  let url; try { url = new URL(value.endpoint); } catch {}
  if (typeof value.endpoint !== "string" || value.endpoint.length > 4096 || url?.protocol !== "https:"
    || url.username || url.password || url.port || url.hash || !config.endpointHosts.has(url.hostname)) {
    throw new HttpError(400, "The subscription endpoint must use an allowed HTTPS push provider.");
  }
  exactKeys(value.keys, ["p256dh", "auth"]);
  if (!token(value.keys.p256dh, 87) || Buffer.from(value.keys.p256dh, "base64url")[0] !== 4 || !token(value.keys.auth, 22)) {
    throw new HttpError(400, "Invalid push subscription keys.");
  }
  if (value.expirationTime != null && (!Number.isSafeInteger(value.expirationTime) || value.expirationTime < 0)) {
    throw new HttpError(400, "Invalid subscription expiry.");
  }
  return { endpoint: url.href, keys: { p256dh: value.keys.p256dh, auth: value.keys.auth }, expirationTime: value.expirationTime ?? null };
}
export function applicationScope(value, origin) {
  let url; try { url = new URL(value); } catch {}
  if (!url || url.origin !== origin || url.username || url.password || url.search || url.hash
    || !url.pathname.endsWith("/") || url.href.length > 2048) throw new HttpError(400, "The app scope must match the requesting origin.");
  return url.href;
}

// An exact provider host allowlist is the first gate. DNS is checked again on
// each socket connection, so a configured host cannot resolve to a private API.
export function isPublicAddress(address) {
  if (isIP(address) === 4) {
    const [a, b, c] = address.split(".").map(Number);
    return !(a === 0 || a === 10 || a === 127 || a >= 224 || (a === 100 && b >= 64 && b <= 127)
      || (a === 169 && b === 254) || (a === 172 && b >= 16 && b <= 31) || (a === 192 && b === 168)
      || (a === 192 && b === 0) || (a === 192 && b === 88 && c === 99)
      || (a === 198 && (b === 18 || b === 19)) || (a === 198 && b === 51 && c === 100)
      || (a === 203 && b === 0 && c === 113));
  }
  if (isIP(address) === 6) {
    const lower = address.toLowerCase();
    // Global unicast only; exclude documentation, Teredo, 6to4 and benchmark
    // prefixes. In particular, mapped IPv4 addresses never bypass the IPv4 gate.
    if (lower.includes(".")) return false;
    const [left, right] = lower.split("::"), start = left ? left.split(":") : [], end = right ? right.split(":") : [];
    const words = [...start, ...Array(8 - start.length - end.length).fill("0"), ...end].map(word => Number.parseInt(word, 16));
    const [first, second] = words;
    return first >= 0x2000 && first <= 0x3fff && first !== 0x2002 && first !== 0x3fff
      && !(first === 0x2001 && (second === 0 || second === 2 || second === 0x0db8 || (second & 0xfff0) === 0x10 || (second & 0xfff0) === 0x20));
  }
  return false;
}
export function safeAgent(resolve = lookup) {
  return new Agent({ keepAlive: true, maxSockets: 8, lookup(hostname, options, callback) {
    resolve(hostname, { all: true, verbatim: true }, (error, records) => {
      if (error) return callback(error);
      if (!records?.length || records.some(record => !isPublicAddress(record.address))) {
        return callback(new Error("Push provider resolved to a non-public address."));
      }
      const family = typeof options === "number" ? options : options?.family;
      const eligible = family ? records.filter(record => record.family === family) : records;
      if (!eligible.length) return callback(new Error("Push provider has no address in the requested family."));
      if (options?.all) callback(null, eligible);
      else callback(null, eligible[0].address, eligible[0].family);
    });
  } });
}

export class RateLimiter {
  constructor({ maximum = 10_000, now = Date.now } = {}) { this.entries = new Map(); this.maximum = maximum; this.now = now; }
  take(key, limit, periodMs = 60_000) {
    const now = this.now();
    let entry = this.entries.get(key);
    if (!entry || entry.until <= now) {
      if (this.entries.size >= this.maximum) {
        for (const [id, item] of this.entries) if (item.until <= now) this.entries.delete(id);
        if (this.entries.size >= this.maximum) throw new HttpError(429, "Service rate limit reached. Try again later.");
      }
      entry = { until: now + periodMs, count: 0 }; this.entries.set(key, entry);
    }
    if (++entry.count > limit) throw new HttpError(429, "Too many requests. Try again later.");
  }
}
