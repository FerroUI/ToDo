import test from "node:test";
import assert from "node:assert/strict";
import { workerFixture } from "./tests/service-worker-fixture.mjs";

test("installation bypasses warmed HTTP entries, checks integrity, and does not activate over editable tabs", async () => {
  const api = await workerFixture();
  api.httpCache.set(api.main, "old module");
  await api.lifecycle("install");
  assert.equal(await api.stores.get(api.cacheName).get(api.main).text(), "current module");
  assert(api.calls.every(request => request.cache === "reload"));
  assert.equal(api.skips, 0);
});

test("an incomplete deployment cannot become a cached release or remove an older installation", async () => {
  const api = await workerFixture();
  const previous = api.prefix + "b".repeat(16);
  api.stores.set(previous, new Map());
  api.network.set(api.main, "wrong deployment");
  await assert.rejects(api.lifecycle("install"), /Integrity mismatch/);
  assert(!api.stores.has(api.cacheName));
  assert(api.stores.has(previous));
});

test("online navigation retains its auth query without overwriting the verified offline shell", async () => {
  const api = await workerFixture(); await api.lifecycle("install");
  const callback = `${api.index}?code=one-use&state=original&demo=1`;
  api.network.set(callback, "new deployment shell");
  const response = await api.request(callback, { mode: "navigate" });
  assert.equal(await response.text(), "new deployment shell");
  assert.deepEqual(api.calls.at(-1), { url: callback, cache: "no-cache" });
  const entries = api.stores.get(api.cacheName);
  assert.equal(await entries.get(api.index).text(), "current shell");
  assert(!entries.has(callback));
});

test("offline document navigation uses the verified shell; arbitrary paths do not turn into the app", async () => {
  const api = await workerFixture(); await api.lifecycle("install"); api.offline = true;
  assert.equal(await (await api.request(`${api.scope}?demo=1`, { mode: "navigate" })).text(), "current shell");
  await assert.rejects(api.request(`${api.scope}missing.html`, { mode: "navigate" }), /Offline/);
});

test("nooffline bypasses both stored responses and the HTTP cache for navigation and its subresources", async () => {
  const api = await workerFixture(); await api.lifecycle("install");
  const preview = `${api.index}?demo=1&nooffline=1`;
  api.network.set(preview, "preview shell");
  api.httpCache.set(api.main, "stale HTTP module");
  api.clients.set("preview", { url: preview });
  assert.equal(await (await api.request(preview, { mode: "navigate" })).text(), "preview shell");
  api.network.set(api.main, "fresh preview module");
  assert.equal(await (await api.request(api.main, { clientId: "preview" })).text(), "fresh preview module");
  assert(api.calls.slice(-2).every(request => request.cache === "no-store"));
  assert.equal(await api.stores.get(api.cacheName).get(api.main).text(), "current module");
  api.offline = true;
  await assert.rejects(api.request(api.main, { clientId: "preview" }), /Offline/);
});

test("query variants and unrelated URLs cannot read or poison the immutable release cache", async () => {
  const api = await workerFixture(); await api.lifecycle("install");
  const variant = `${api.main}?different=1`;
  api.network.set(variant, "query response");
  assert.equal(await (await api.request(variant)).text(), "query response");
  assert(!api.stores.get(api.cacheName).has(variant));
  assert.equal(await (await api.request(api.main)).text(), "current module");
  assert.equal(api.request("https://example.test/Other/main.js"), undefined);
  assert.equal(api.request(api.main, { method: "POST" }), undefined);
});

test("a waiting release serves its own immutable URLs; activation only removes this app's old cache", async () => {
  const api = await workerFixture(); await api.lifecycle("install");
  const version = "b".repeat(16), waiting = api.prefix + version;
  const asset = `${api.scope}_app/${version}/main.js`;
  api.stores.set(waiting, new Map([[asset, new Response("waiting module")]]));
  api.stores.set("ferroui-todo-v2-%2FOther%2F-previous", new Map());
  api.stores.set("ferroui-todo-legacy-unscoped", new Map());
  api.offline = true;
  assert.equal(await (await api.request(asset)).text(), "waiting module");
  await api.lifecycle("activate");
  assert(!api.stores.has(waiting));
  assert(api.stores.has(api.cacheName));
  assert(api.stores.has("ferroui-todo-v2-%2FOther%2F-previous"));
  assert(api.stores.has("ferroui-todo-legacy-unscoped"));
  assert.equal(api.claims, 1);
});
