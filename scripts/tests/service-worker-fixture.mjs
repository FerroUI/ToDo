import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import vm from "node:vm";

export const integrity = body => `sha256-${createHash("sha256").update(body).digest("base64")}`;

export async function workerFixture() {
  const scope = "https://example.test/ToDo/", version = "a".repeat(16);
  const prefix = `ferroui-todo-v2-${encodeURIComponent(new URL(scope).pathname)}-`;
  const cacheName = prefix + version;
  const main = new URL(`_app/${version}/main.js`, scope).href;
  const index = new URL("index.html", scope).href;
  const assets = [[main, integrity("current module")], [index, integrity("current shell")]];
  const stores = new Map(), network = new Map([[main, "current module"], [index, "current shell"]]);
  const httpCache = new Map(), clients = new Map(), calls = [], events = new Map();
  const api = { scope, version, prefix, cacheName, main, index, stores, network, httpCache, clients, calls, claims: 0, skips: 0, offline: false };
  const key = request => typeof request === "string" ? request : request.url;
  const fetch = async (request, options) => {
    const url = key(request), mode = options?.cache ?? request.cache;
    calls.push({ url, cache: mode });
    if (api.offline) throw new Error("Offline");
    const body = !["reload", "no-store", "no-cache"].includes(mode) && httpCache.has(url) ? httpCache.get(url) : network.get(url);
    return new Response(body ?? "Not found", { status: body === undefined ? 404 : 200 });
  };
  const cache = entries => ({
    match: async request => entries.get(key(request))?.clone(),
    addAll: async requests => {
      // Cache.addAll commits only after every fetch succeeds. Native fetch
      // performs the integrity check; emulate that boundary without a browser.
      const responses = await Promise.all(requests.map(async request => {
        const response = await fetch(request);
        if (!response.ok || integrity(await response.clone().text()) !== request.integrity) throw new Error("Integrity mismatch");
        return [request.url, response];
      }));
      for (const [url, response] of responses) entries.set(url, response);
    },
  });
  const caches = {
    keys: async () => [...stores.keys()],
    delete: async name => stores.delete(name),
    open: async name => { if (!stores.has(name)) stores.set(name, new Map()); return cache(stores.get(name)); },
    match: async (request, options) => stores.get(options.cacheName)?.get(key(request))?.clone(),
  };
  const self = {
    registration: { scope },
    clients: { get: async id => clients.get(id), claim: async () => api.claims++, matchAll: async () => [...clients.values()] },
    skipWaiting: () => api.skips++,
    addEventListener: (type, handler) => events.set(type, handler),
  };
  const template = await readFile(new URL("../../crates/todo-browser/wwwroot/app-sw.js", import.meta.url), "utf8");
  vm.runInNewContext(template.replace('"__BUILD_VERSION__"', JSON.stringify(version)).replace('"__PRECACHE_FILES__"', JSON.stringify(assets)), {
    self, caches, fetch, Request, Response, URL,
  });
  api.lifecycle = type => { let result; events.get(type)({ waitUntil: promise => result = promise }); return result; };
  api.request = (url, { mode = "cors", method = "GET", clientId = "" } = {}) => {
    let result;
    events.get("fetch")({ request: { url, mode, method }, clientId, respondWith: promise => result = promise });
    return result;
  };
  api.message = event => events.get("message")(event);
  return api;
}
