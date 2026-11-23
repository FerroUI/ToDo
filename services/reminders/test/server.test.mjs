import test from "node:test";
import assert from "node:assert/strict";
import { createReminderServer } from "../src/server.mjs";
import { fixture, origin, random, subscription } from "./fixture.mjs";

async function serverFixture(t, overrides) {
  const api = await fixture(overrides), server = createReminderServer(api.store, api.config, { now: api.now });
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
  t.after(() => new Promise(resolve => server.close(resolve)));
  const base = `http://127.0.0.1:${server.address().port}`;
  return { ...api, base, request: (path, options = {}) => fetch(base + path, { ...options, headers: {
    origin, "content-type": "application/json", authorization: `Bearer ${api.enrollment.capability}`, ...(options.headers || {}),
  } }) };
}

test("HTTP API requires exact origin and enrollment key, exposes only public configuration and scoped CORS", async t => {
  const api = await serverFixture(t);
  const publicConfig = await api.request("/v1/config"); assert.equal(publicConfig.status, 200);
  assert.equal(publicConfig.headers.get("access-control-allow-origin"), origin);
  const config = await publicConfig.json(); assert.equal(config.publicKey, api.config.vapid.publicKey);
  assert(!JSON.stringify(config).includes(api.config.vapid.privateKey));
  assert.equal((await fetch(api.base + "/v1/config")).status, 403);
  const denied = await api.request("/v1/config", { headers: { origin: "https://evil.test" } });
  assert.equal(denied.status, 403); assert.equal(denied.headers.get("access-control-allow-origin"), null);
  const enrollment = { id: random(16), capability: random(), scope: `${origin}/ToDo/`, subscription: subscription() };
  assert.equal((await api.request("/v1/installations", { method: "POST", body: JSON.stringify(enrollment) })).status, 403);
  const allowed = await api.request("/v1/installations", { method: "POST", headers: { "x-enrollment-key": api.config.enrollmentKey }, body: JSON.stringify(enrollment) });
  assert.equal(allowed.status, 201); assert.equal((await allowed.json()).revision, 0);
  const preflight = await api.request("/v1/installations", { method: "OPTIONS" });
  assert.equal(preflight.status, 204); assert(preflight.headers.get("access-control-allow-headers").includes("If-Match"));
});

test("HTTP CAS, authorization, content limits and deletion preserve the durable authoritative schedule", async t => {
  const api = await serverFixture(t, { maxBodyBytes: 2048 }), path = `/v1/installations/${api.enrollment.id}`;
  assert.equal((await api.request(path, { headers: { authorization: `Bearer ${random()}` } })).status, 401);
  const job = api.job(), body = JSON.stringify({ workspace: api.workspace, jobs: [job] });
  assert.equal((await api.request(path, { method: "PUT", body })).status, 428);
  let response = await api.request(path, { method: "PUT", headers: { "if-match": '"0"' }, body });
  assert.equal(response.status, 200); assert.equal(response.headers.get("etag"), '"1"');
  response = await api.request(path, { method: "PUT", headers: { "if-match": '"0"' }, body });
  assert.equal(response.status, 409); assert.equal((await response.json()).revision, 1);
  assert.equal((await api.request(path, { method: "PUT", headers: { "if-match": '"1"' }, body: " ".repeat(2049) })).status, 413);
  assert.equal((await api.request(path, { method: "PUT", headers: { "content-type": "text/plain" }, body })).status, 415);
  assert.equal((await api.request(path + "?capability=forbidden")).status, 400);
  assert.equal((await api.request(path, { method: "DELETE" })).status, 204);
  assert.equal((await api.request(path)).status, 401);
  assert.deepEqual(api.store.snapshot().installations, {});
});
