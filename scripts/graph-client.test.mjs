import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";

registerHooks({
  resolve(specifier, context, next) {
    const result = next(specifier, context);
    return result.url.endsWith("/cloud/auth.js") ? { url: "fixture:auth", shortCircuit: true } : result;
  },
  load(url, context, next) {
    if (url !== "fixture:auth") return next(url, context);
    return { format: "module", shortCircuit: true, source: `export const accessToken=async renew=>{globalThis.graphTransport.tokens.push(renew);return 'test-only-token';};` };
  },
});
const { graph, graphBytes, graphUpload, collection } = await import("../crates/todo-browser/wwwroot/cloud/client.js");
const json = (value, status = 200, headers) => new Response(JSON.stringify(value), { status, headers });
function fixture(responses) {
  const state = globalThis.graphTransport = { tokens: [], requests: [] };
  globalThis.fetch = async (url, options) => {
    state.requests.push({ url: String(url), ...options });
    if (!responses.length) throw new Error("Unexpected additional network request");
    return responses.shift();
  };
  return state;
}

test("Graph credentials never follow foreign, credentialed, or non-v1 continuation URLs", async () => {
  const state = fixture([]);
  for (const path of ["https://attacker.test/v1.0/me", "https://user:pass@graph.microsoft.com/v1.0/me", "https://graph.microsoft.com/beta/me", "https://graph.microsoft.com/v1.0/me#fragment"]) {
    await assert.rejects(graph(path), /unexpected continuation/);
  }
  assert.equal(state.tokens.length, 0); assert.equal(state.requests.length, 0);
});

test("ambiguous POST failures are not automatically replayed", async () => {
  const state = fixture([json({ error: { message: "Unavailable", code: "ServiceUnavailable" } }, 503)]);
  await assert.rejects(graph("me/todo/lists", "POST", { displayName: "Keep once" }), error => error.status === 503 && error.code === "ServiceUnavailable");
  assert.equal(state.requests.length, 1);
});

test("authorization renewal is distinct from throttling retries", async t => {
  t.mock.method(globalThis, "setTimeout", callback => { callback(); return 0; });
  const state = fixture([json({}, 401), json({}, 429, { "Retry-After": "1" }), json({ id: "ok" })]);
  assert.equal((await graph("me/todo/lists/list")).id, "ok");
  assert.deepEqual(state.tokens, [false, true, false]);
  assert(state.requests.every(request => request.redirect === "error" && request.credentials === "omit" && request.cache === "no-store"));
});

test("binary download uses Graph authorization and does not JSON-decode file bytes", async () => {
  const state = fixture([new Response(new Uint8Array([0, 1, 255]), { headers: { "Content-Type": "application/octet-stream", "Content-Length": "3" } })]);
  const bytes = await graphBytes("me/todo/lists/list/tasks/task/attachments/file/$value");
  assert.deepEqual([...new Uint8Array(await bytes.arrayBuffer())], [0, 1, 255]);
  assert.equal(state.requests[0].headers.Authorization, "Bearer test-only-token");
  assert.equal(state.requests[0].headers.Accept, "*/*");
});

test("To Do upload ranges append content and authorize only a Graph attachment session", async () => {
  const state = fixture([new Response(null, { status: 201, headers: { Location: "https://graph.microsoft.com/v1.0/me/todo/lists/list/tasks/task/attachments/file" } })]);
  const result = await graphUpload("https://graph.microsoft.com/v1.0/users/user/todo/lists/list/tasks/task/attachmentSessions/session", new Blob(["abc"]), 2, 5);
  assert.equal(result.status, 201);
  assert.match(state.requests[0].url, /attachmentSessions\/session\/content$/);
  assert.equal(state.requests[0].headers.Authorization, "Bearer test-only-token");
  assert.equal(state.requests[0].headers["Content-Range"], "bytes 2-4/5");
  assert.equal(state.requests[0].headers["Content-Type"], "application/octet-stream");
  assert.equal(state.requests[0].headers["Content-Length"], undefined);
  await assert.rejects(graphUpload("https://attacker.test/session", new Blob(["abc"]), 0, 3), /unexpected continuation/);
  assert.equal(state.requests.length, 1);
});

test("collections reject malformed pages and canonical repeated next links", async () => {
  fixture([json({ value: [], "@odata.nextLink": "https://graph.microsoft.com/v1.0/me/todo/lists" })]);
  await assert.rejects(collection("me/todo/lists"), /repeated/);
  fixture([json({ value: null })]);
  await assert.rejects(collection("me/todo/lists"), /invalid collection/);
});
