const VERSION = "__BUILD_VERSION__";
const ASSETS = "__PRECACHE_FILES__";
const CACHE = `ferroui-todo-${VERSION}`;

self.addEventListener("install", event => {
  if (Array.isArray(ASSETS)) event.waitUntil(caches.open(CACHE).then(cache => cache.addAll(ASSETS)));
  self.skipWaiting();
});

self.addEventListener("activate", event => event.waitUntil((async () => {
  for (const name of await caches.keys()) {
    if (name.startsWith("ferroui-todo-") && name !== CACHE) await caches.delete(name);
  }
  await self.clients.claim();
})()));

self.addEventListener("fetch", event => {
  const request = event.request;
  const url = new URL(request.url);
  if (request.method !== "GET" || url.origin !== self.location.origin || !url.href.startsWith(self.registration.scope)) return;
  event.respondWith((async () => {
    const cache = await caches.open(CACHE);
    const cached = await cache.match(request, { ignoreSearch: true });
    if (cached && request.mode !== "navigate") return cached;
    try {
      const response = await fetch(request);
      if (response.ok) {
        // OAuth callbacks can carry a one-use authorization code in the query.
        // Cache the static navigation document under its clean URL only.
        const key = request.mode === "navigate" ? new URL("./index.html", self.registration.scope).href : request;
        await cache.put(key, response.clone());
      }
      return response;
    } catch (error) {
      return cached || (request.mode === "navigate" ? await cache.match("./index.html") : undefined) || Response.error();
    }
  })());
});

self.addEventListener("notificationclick", event => {
  event.notification.close();
  event.waitUntil((async () => {
    const clients = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
    const existing = clients.find(client => client.url.startsWith(self.registration.scope));
    if (existing) await existing.focus();
    else await self.clients.openWindow(self.registration.scope);
  })());
});
