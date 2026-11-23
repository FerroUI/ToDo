import { createServer } from "node:http";
import { HttpError, hash, equalSecret, RateLimiter } from "./security.mjs";
import { Schedules } from "./schedules.mjs";

async function body(request, limit) {
  if (request.headers["content-type"]?.split(";")[0].trim().toLowerCase() !== "application/json"
    || (request.headers["content-encoding"] && request.headers["content-encoding"] !== "identity")) throw new HttpError(415, "Use an uncompressed JSON request.");
  if (Number(request.headers["content-length"]) > limit) throw new HttpError(413, "Request is too large.");
  let length = 0, chunks = [];
  for await (const chunk of request) {
    length += chunk.length;
    if (length > limit) throw new HttpError(413, "Request is too large.");
    chunks.push(chunk);
  }
  try { return JSON.parse(Buffer.concat(chunks).toString("utf8")); }
  catch { throw new HttpError(400, "Request is not valid JSON."); }
}

export function createReminderServer(store, config, { now = Date.now, onError = () => {} } = {}) {
  const schedules = new Schedules(store, config, now), limiter = new RateLimiter({ now }), enrollmentHash = hash(config.enrollmentKey);
  const server = createServer({ maxHeaderSize: 8192, requestTimeout: 15_000, headersTimeout: 10_000 }, (request, response) => {
    const reply = (status, data) => {
      response.writeHead(status, { "content-type": "application/json; charset=utf-8", "cache-control": "no-store",
        "x-content-type-options": "nosniff", ...(status === 429 ? { "retry-after": "60" } : {}) });
      response.end(status === 204 ? undefined : JSON.stringify(data));
    };
    void (async () => {
      const url = new URL(request.url, "http://service.invalid");
      if (url.pathname === "/health" && request.method === "GET" && !url.search) return reply(store.poisoned ? 503 : 200, { ok: !store.poisoned });
      const origin = request.headers.origin;
      if (typeof origin !== "string" || !config.origins.has(origin)) throw new HttpError(403, "This application origin is not allowed.");
      response.setHeader("access-control-allow-origin", origin);
      response.setHeader("vary", "Origin");
      response.setHeader("access-control-expose-headers", "ETag, Retry-After");
      const address = request.socket.remoteAddress || "unknown"; // Ignore spoofable forwarded headers.
      limiter.take(`ip:${address}`, 240);
      if (url.search || url.username || url.password) throw new HttpError(400, "API URLs cannot contain query parameters or credentials.");
      if (request.method === "OPTIONS") {
        response.setHeader("access-control-allow-methods", "GET, POST, PUT, DELETE, OPTIONS");
        response.setHeader("access-control-allow-headers", "Content-Type, Authorization, If-Match, X-Enrollment-Key");
        response.setHeader("access-control-max-age", "600");
        return reply(204);
      }
      if (url.pathname === "/v1/config" && request.method === "GET") return reply(200, {
        version: 1, publicKey: config.vapid.publicKey, enrollmentRequired: true, maxJobs: config.maxJobs,
        horizonMs: config.horizonMs, expiryMs: config.expiryMs, leaseMs: config.leaseMs,
      });
      if (url.pathname === "/v1/installations" && request.method === "POST") {
        limiter.take(`enroll:${address}`, 10, 60 * 60_000);
        if (!equalSecret(request.headers["x-enrollment-key"], enrollmentHash)) throw new HttpError(403, "Invalid enrollment key.");
        return reply(201, await schedules.enroll(await body(request, config.maxBodyBytes), origin));
      }
      const match = /^\/v1\/installations\/([A-Za-z0-9_-]{22})$/.exec(url.pathname);
      if (!match) throw new HttpError(404, "Unknown endpoint.");
      const authorization = request.headers.authorization || "";
      if (!/^Bearer [A-Za-z0-9_-]{43}$/.test(authorization)) throw new HttpError(401, "An installation capability is required.");
      const capability = authorization.slice(7), id = match[1];
      // Authenticate before consuming a per-installation bucket: random callers
      // cannot exhaust another browser's update quota merely by knowing its ID.
      schedules.get(id, capability, origin);
      limiter.take(`installation:${id}`, 120);
      if (request.method === "GET") {
        const result = schedules.get(id, capability, origin);
        response.setHeader("etag", `"${result.revision}"`); return reply(200, result);
      }
      if (request.method === "PUT") {
        const result = await schedules.update(id, capability, origin, request.headers["if-match"], await body(request, config.maxBodyBytes));
        response.setHeader("etag", `"${result.revision}"`); return reply(200, result);
      }
      if (request.method === "DELETE") { await schedules.remove(id, capability, origin); return reply(204); }
      throw new HttpError(405, "Unsupported method.");
    })().catch(error => {
      if (!(error instanceof HttpError)) onError(error);
      if (!response.headersSent) reply(error.status || 500, { error: error instanceof HttpError ? error.message : "Reminder service is unavailable.", ...(error.details || {}) });
      else response.destroy();
    });
  });
  server.maxRequestsPerSocket = 100;
  server.on("clientError", (_error, socket) => { socket.end("HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n"); });
  return server;
}
