import { resolve } from "node:path";
import { isIP } from "node:net";

export const DAY = 86_400_000;
export const DEFAULT_PUSH_HOSTS = ["fcm.googleapis.com", "updates.push.services.mozilla.com", "web.push.apple.com"];

function number(value, fallback, min, max, name) {
  const parsed = value == null || value === "" ? fallback : Number(value);
  if (!Number.isSafeInteger(parsed) || parsed < min || parsed > max) throw new Error(`${name} must be ${min}–${max}.`);
  return parsed;
}

export function readConfig(env = process.env) {
  const development = env.TODO_PUSH_ALLOW_LOCAL_HTTP === "1";
  const origins = (env.TODO_PUSH_ORIGINS || "").split(",").map(value => value.trim()).filter(Boolean);
  if (!origins.length || origins.length > 32) throw new Error("Configure TODO_PUSH_ORIGINS with exact application origins.");
  for (const value of origins) {
    const url = new URL(value);
    const local = ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname);
    if (url.origin !== value || url.username || url.password || (url.protocol !== "https:" && !(development && local && url.protocol === "http:"))) {
      throw new Error("Application origins must be exact HTTPS origins (without a path or wildcard).");
    }
  }
  const enrollmentKey = env.TODO_PUSH_ENROLLMENT_KEY || "";
  if (!/^[A-Za-z0-9_-]{43,128}$/.test(enrollmentKey)) throw new Error("TODO_PUSH_ENROLLMENT_KEY must be a random base64url key of at least 32 bytes.");
  const publicKey = env.TODO_PUSH_VAPID_PUBLIC_KEY || "", privateKey = env.TODO_PUSH_VAPID_PRIVATE_KEY || "";
  if (Buffer.from(publicKey, "base64url").length !== 65 || Buffer.from(publicKey, "base64url")[0] !== 4
    || Buffer.from(privateKey, "base64url").length !== 32) throw new Error("Configure valid VAPID keys; run npm run keys.");
  const subject = env.TODO_PUSH_VAPID_SUBJECT || "";
  if (!/^mailto:[^\s@]+@[^\s@]+\.[^\s@]+$/.test(subject)) {
    let url; try { url = new URL(subject); } catch {}
    if (url?.protocol !== "https:" || url.username || url.password || url.hostname === "localhost") {
      throw new Error("TODO_PUSH_VAPID_SUBJECT must be a real contact mailto: address or public HTTPS URL.");
    }
  }
  const extraHosts = (env.TODO_PUSH_EXTRA_ENDPOINT_HOSTS || "").split(",").map(value => value.trim().toLowerCase()).filter(Boolean);
  if (extraHosts.length > 16 || extraHosts.some(host => !/^[a-z0-9](?:[a-z0-9.-]{0,251}[a-z0-9])?$/.test(host)
    || !host.includes(".") || host.includes("..") || isIP(host))) throw new Error("Additional push endpoint hosts must be exact DNS host names.");
  return Object.freeze({
    host: env.TODO_PUSH_HOST || "127.0.0.1",
    port: number(env.TODO_PUSH_PORT, 8787, 0, 65535, "Port"),
    path: resolve(env.TODO_PUSH_DATA_FILE || "data/reminders.json"),
    origins: new Set(origins), enrollmentKey,
    vapid: { publicKey, privateKey, subject },
    endpointHosts: new Set([...DEFAULT_PUSH_HOSTS, ...extraHosts]),
    maxInstallations: number(env.TODO_PUSH_MAX_INSTALLATIONS, 1000, 1, 10000, "Installation limit"),
    maxJobs: number(env.TODO_PUSH_MAX_JOBS, 200, 1, 1000, "Reminder limit"),
    horizonMs: number(env.TODO_PUSH_HORIZON_DAYS, 366, 1, 730, "Scheduling horizon") * DAY,
    leaseMs: number(env.TODO_PUSH_LEASE_DAYS, 366, 1, 730, "Subscription lease") * DAY,
    expiryMs: number(env.TODO_PUSH_EXPIRY_MINUTES, 60, 1, 1440, "Reminder expiry") * 60_000,
    maxBodyBytes: 256 * 1024, maxAttempts: 8, requestTimeoutMs: 10_000,
  });
}
