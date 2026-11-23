export function serviceUrl(value) {
  let url; try { url = new URL(String(value).trim()); } catch {}
  const local = url && ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname);
  if (!url || url.username || url.password || url.search || url.hash || url.href.length > 2048
    || (url.protocol !== "https:" && !(local && url.protocol === "http:"))) throw new Error("Enter the HTTPS address of your reminder service.");
  if (!url.pathname.endsWith("/")) url.pathname += "/";
  return url.href;
}
export function publicKey(value) {
  if (typeof value !== "string" || !/^B[A-Za-z0-9_-]{86}$/.test(value)) throw new Error("Enter the service's public VAPID key.");
  const bytes = Uint8Array.from(atob(value.replaceAll("-", "+").replaceAll("_", "/") + "="), character => character.charCodeAt(0));
  if (bytes.length !== 65 || bytes[0] !== 4) throw new Error("The public VAPID key is invalid.");
  return bytes;
}
async function readJson(response) {
  if (response.status === 204) return null;
  if (Number(response.headers.get("content-length")) > 64 * 1024) throw new Error("The reminder service response is too large.");
  const reader = response.body?.getReader();
  if (!reader) throw new Error("The reminder service did not return JSON.");
  const chunks = []; let length = 0;
  try {
    for (;;) {
      const { done, value } = await reader.read(); if (done) break;
      length += value.length; if (length > 64 * 1024) throw new Error("The reminder service response is too large.");
      chunks.push(value);
    }
  } catch (error) { await reader.cancel().catch(() => {}); throw error; }
  const joined = new Uint8Array(length); let offset = 0;
  for (const chunk of chunks) { joined.set(chunk, offset); offset += chunk.length; }
  try { return JSON.parse(new TextDecoder().decode(joined)); }
  catch { throw new Error("The reminder service did not return valid JSON."); }
}
export async function serviceRequest(base, path, { method = "GET", capability, revision, enrollment, body, fetcher = fetch } = {}) {
  const headers = { accept: "application/json" };
  if (body !== undefined) headers["content-type"] = "application/json";
  if (capability) headers.authorization = `Bearer ${capability}`;
  if (revision != null) headers["if-match"] = `"${revision}"`;
  if (enrollment) headers["x-enrollment-key"] = enrollment;
  const response = await fetcher(new URL(path, serviceUrl(base)), { method, headers, body: body === undefined ? undefined : JSON.stringify(body),
    mode: "cors", credentials: "omit", cache: "no-store", redirect: "error", referrerPolicy: "no-referrer", signal: AbortSignal.timeout(15_000) });
  const result = await readJson(response);
  if (!response.ok) throw Object.assign(new Error(typeof result?.error === "string" ? result.error.slice(0, 500) : "The reminder service rejected this request."),
    { status: response.status, revision: result?.revision });
  return result;
}
export function serviceConfig(value, expectedKey) {
  if (value?.version !== 1 || value.publicKey !== expectedKey || !value.enrollmentRequired
    || !Number.isSafeInteger(value.maxJobs) || value.maxJobs < 1 || value.maxJobs > 1000
    || !Number.isSafeInteger(value.horizonMs) || value.horizonMs < 86_400_000 || value.horizonMs > 730 * 86_400_000
    || !Number.isSafeInteger(value.leaseMs) || value.leaseMs < 86_400_000 || value.leaseMs > 730 * 86_400_000
    || !Number.isSafeInteger(value.expiryMs) || value.expiryMs < 60_000 || value.expiryMs > 86_400_000) {
    throw new Error("The reminder service configuration or public key does not match. Check its setup.");
  }
  publicKey(value.publicKey);
  return { version: 1, publicKey: value.publicKey, maxJobs: value.maxJobs, horizonMs: value.horizonMs, expiryMs: value.expiryMs, leaseMs: value.leaseMs };
}
