import test from "node:test";
import assert from "node:assert/strict";
import { prepareOfflinePreview, registerOffline } from "../crates/todo-browser/wwwroot/offline.js";

function environment({ aware = false, foreign = false } = {}) {
  const data = new Map();
  const storage = { getItem: key => data.get(key), setItem: (key, value) => data.set(key, value), removeItem: key => data.delete(key) };
  const api = { href: "https://example.test/ToDo/index.html?nooffline=1&demo=1#state", storage: () => storage, reloads: 0, removals: 0, data };
  api.reload = () => api.reloads++;
  const scope = foreign ? "https://example.test/" : "https://example.test/ToDo/";
  api.serviceWorker = {
    controller: {
      scriptURL: `${scope}app-sw.js`,
      postMessage: (_message, [port]) => { if (aware) port.postMessage({ type: "TODO_PREVIEW_READY" }); },
    },
    getRegistration: async () => ({ scope, unregister: async () => { api.removals++; return true; } }),
  };
  return api;
}

test("preview-aware workers keep other tabs installed and do not reload the page", async () => {
  const api = environment({ aware: true });
  assert(await prepareOfflinePreview(api));
  assert.equal(api.reloads, 0); assert.equal(api.removals, 0);
});

test("a legacy worker is unregistered only for this app, before one bounded preview reload", async () => {
  const api = environment(); const href = api.href;
  assert.equal(await prepareOfflinePreview(api, 0), false);
  assert.equal(api.reloads, 1); assert.equal(api.removals, 1); assert.equal(api.href, href);
  assert.deepEqual([...api.data.values()], ["1"]);
  await assert.rejects(prepareOfflinePreview(api, 0), /still controls/);
  assert.equal(api.reloads, 1); assert.equal(api.removals, 1);
  api.serviceWorker.controller = null;
  assert(await prepareOfflinePreview(api));
  assert.equal(api.data.size, 0);
});

test("a broader registration belonging to another app is never removed", async () => {
  const api = environment({ foreign: true });
  await assert.rejects(prepareOfflinePreview(api, 0), /another app/);
  assert.equal(api.removals, 0); assert.equal(api.reloads, 0);
});

test("a controller change during the handoff never unregisters the replacement worker", async () => {
  const api = environment();
  api.serviceWorker.getRegistration = async () => {
    api.serviceWorker.controller = { scriptURL: "https://example.test/ToDo/app-sw.js" };
    return { scope: "https://example.test/ToDo/", unregister: async () => api.removals++ };
  };
  await assert.rejects(prepareOfflinePreview(api, 0), /installation changed/);
  assert.equal(api.removals, 0); assert.equal(api.reloads, 0);
});

test("offline registration uses document scope, ignores HTTP worker caches, and skips previews", async () => {
  const calls = [];
  const serviceWorker = { register: async (...args) => { calls.push(args); return "registered"; } };
  assert.equal(await registerOffline(serviceWorker, "https://example.test/ToDo/index.html?code=once#fragment"), "registered");
  assert.deepEqual(calls, [["https://example.test/ToDo/app-sw.js", { scope: "https://example.test/ToDo/", updateViaCache: "none" }]]);
  assert.equal(await registerOffline(serviceWorker, "https://example.test/ToDo/?nooffline=1"), null);
  assert.equal(calls.length, 1);
});
