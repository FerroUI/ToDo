const VERSION = "__BUILD_VERSION__";
const ASSETS = "__PRECACHE_FILES__";
const SCOPE = new URL(self.registration.scope);
const PREFIX = `ferroui-todo-v2-${encodeURIComponent(SCOPE.pathname)}-`;
const CACHE = PREFIX + VERSION;
const INDEX = new URL("index.html", SCOPE).href;
const resources = new Set(Array.isArray(ASSETS) ? ASSETS.map(([url]) => new URL(url, SCOPE).href) : []);

self.addEventListener("install", event => event.waitUntil((async () => {
  if (!Array.isArray(ASSETS)) throw new Error("Offline resources were not prepared.");
  try {
    const cache = await caches.open(CACHE);
    // A warmed HTTP cache must not seed this version with another deployment's
    // bytes. Integrity also rejects partial deployments before activation.
    await cache.addAll(ASSETS.map(([url, integrity]) => new Request(new URL(url, SCOPE), {
      cache: "reload", credentials: "same-origin", integrity,
    })));
  } catch (error) { await caches.delete(CACHE); throw error; }
  // Keep the normal waiting lifecycle: an update never reloads an editable tab.
})()));

self.addEventListener("activate", event => event.waitUntil((async () => {
  for (const name of await caches.keys()) {
    if (name.startsWith(PREFIX) && name !== CACHE) await caches.delete(name);
  }
  await self.clients.claim();
})()));

self.addEventListener("message", event => {
  if (event.data?.type === "TODO_PREVIEW_CHECK") event.ports[0]?.postMessage({ type: "TODO_PREVIEW_READY" });
});

self.addEventListener("fetch", event => {
  const request = event.request;
  const url = new URL(request.url);
  if (request.method !== "GET" || url.origin !== SCOPE.origin || !url.pathname.startsWith(SCOPE.pathname)) return;
  event.respondWith((async () => {
    const client = event.clientId ? await self.clients.get(event.clientId) : null;
    if (url.searchParams.has("nooffline") || (client && new URL(client.url).searchParams.has("nooffline"))) {
      return fetch(request, { cache: "no-store" });
    }
    const navigation = request.mode === "navigate" && (url.pathname === SCOPE.pathname || url.pathname === new URL(INDEX).pathname);
    if (navigation) {
      try {
        // Preserve the actual query/hash processing in the document, including
        // OAuth callbacks. Navigation responses never enter CacheStorage.
        const response = await fetch(request, { cache: "no-cache" });
        if (response.status < 500) return response;
      } catch {}
      return (await caches.open(CACHE)).match(INDEX).then(response => response || Response.error());
    }
    if (!url.search && resources.has(url.href)) {
      const cached = await (await caches.open(CACHE)).match(request);
      if (cached) return cached;
    }
    const revision = /^_app\/([a-f0-9]{16})\//.exec(url.pathname.slice(SCOPE.pathname.length))?.[1];
    if (!url.search && revision) {
      // A newly loaded document may use an already installed, waiting revision.
      // Its immutable URLs can safely use that revision's verified cache.
      const cached = await caches.match(request, { cacheName: PREFIX + revision });
      if (cached) return cached;
    }
    // Only the integrity-checked installation writes caches. Unknown paths and
    // query variants remain ordinary requests instead of aliasing cached files.
    return fetch(request);
  })());
});

self.addEventListener("notificationclick", event => {
  event.notification.close();
  event.waitUntil((async () => {
    const clients = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
    const existing = clients.find(client => client.url.startsWith(SCOPE.href));
    if (existing) await existing.focus();
    else await self.clients.openWindow(SCOPE.href);
  })());
});
