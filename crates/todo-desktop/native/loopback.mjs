import { createServer } from "node:http";
import { randomBytes, timingSafeEqual } from "node:crypto";

export function nativeRedirect(value = "http://localhost/") {
  const url = new URL(value);
  if (url.protocol !== "http:" || !["localhost", "127.0.0.1", "[::1]"].includes(url.hostname)
    || url.username || url.password || url.search || url.hash) throw new Error("The native Microsoft redirect must be an HTTP loopback URL with no query or credentials.");
  return url;
}

export function callbackResult(request, redirect, expectedState) {
  if (request.method !== "GET" || request.headers.host !== redirect.host) return null;
  const url = new URL(request.url, redirect.origin);
  if (url.origin !== redirect.origin || url.pathname !== redirect.pathname) return null;
  const states = url.searchParams.getAll("state");
  if (states.length !== 1) return null;
  const a = Buffer.from(states[0]), b = Buffer.from(expectedState);
  if (a.length !== b.length || !timingSafeEqual(a, b)) return null;
  const codes = url.searchParams.getAll("code"), errors = url.searchParams.getAll("error");
  if (codes.length === 1 && codes[0] && errors.length === 0) return { code: codes[0] };
  if (errors.length === 1 && codes.length === 0) return { error: errors[0] === "access_denied" ? "Microsoft sign-in was cancelled." : "Microsoft could not complete sign-in. Check the application registration and try again." };
  return null;
}

export async function receiveAuthorization({ redirectUri, authorize, open, timeout = 180_000 }) {
  const redirect = nativeRedirect(redirectUri);
  const state = randomBytes(32).toString("base64url");
  let accept, reject, finished = false;
  const result = new Promise((resolve, fail) => { accept = resolve; reject = fail; });
  result.catch(() => {}); // Keep an early cancellation handled while the browser is being opened.
  const server = createServer((request, response) => {
    const parsed = callbackResult(request, redirect, state);
    response.setHeader("Content-Type", "text/plain; charset=utf-8");
    response.setHeader("Cache-Control", "no-store");
    response.setHeader("Referrer-Policy", "no-referrer");
    response.setHeader("Content-Security-Policy", "default-src 'none'; frame-ancestors 'none'");
    if (!parsed || finished) { response.writeHead(400); response.end("This sign-in response is not valid for this To Do session."); return; }
    finished = true;
    response.end(parsed.error ? "Sign-in did not complete. Return to To Do." : "You can close this tab and return to To Do.");
    parsed.error ? reject(new Error(parsed.error)) : accept({ code: parsed.code, redirectUri: redirect.href });
  });
  server.requestTimeout = 5_000; server.headersTimeout = 5_000; server.maxHeadersCount = 32;
  server.maxConnections = 4;
  await new Promise((resolve, fail) => {
    server.once("error", fail);
    server.listen(Number(redirect.port) || 0, redirect.hostname === "[::1]" ? "::1" : "127.0.0.1", () => { server.removeListener("error", fail); resolve(); });
  });
  redirect.port = String(server.address().port);
  const timer = setTimeout(() => reject(new Error("Microsoft sign-in timed out. Connect again when ready.")), timeout);
  try {
    const address = await authorize({ redirectUri: redirect.href, state });
    await open(address);
    return await result;
  } finally {
    clearTimeout(timer);
    server.closeAllConnections();
    await new Promise(resolve => server.close(resolve));
  }
}
