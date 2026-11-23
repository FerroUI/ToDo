// Classic worker companion, loaded from this deployment's immutable assets.
// No Microsoft credentials or task document access is available in this worker.
(() => {
  const token = value => typeof value === "string" && /^[A-Za-z0-9_-]{43}$/.test(value);
  function validPayload(value) {
    return value?.version === 1 && /^[A-Za-z0-9_-]{22}$/.test(value.installation || "") && token(value.workspace) && token(value.id)
      && Number.isSafeInteger(value.at) && Number.isSafeInteger(value.expiresAt) && value.expiresAt > value.at;
  }
  function activeJob(state, payload, now) {
    if (!state?.enabled || state.id !== payload.installation || state.workspace !== payload.workspace || state.expiresAt <= now) return null;
    const job = state.plan?.[payload.id];
    return job && job.at === payload.at && job.expiresAt === payload.expiresAt && job.at <= now + 30_000 && job.expiresAt > now ? job : null;
  }
  async function receive(worker, store, payload, now = Date.now) {
    if (!validPayload(payload) || payload.expiresAt <= now() || payload.at > now() + 30_000) return false;
    const state = await store.read(), job = activeJob(state, payload, now());
    if (!job) return false;
    const receipt = await store.claim({ installation: state.id, workspace: state.workspace, id: job.id,
      occurrence: job.occurrence, taskId: job.taskId, date: job.date, minutes: job.minutes, now: now(), until: job.expiresAt });
    if (!receipt) return false;
    try {
      const current = await store.read(), visible = activeJob(current, payload, now());
      if (!visible) { await store.finish(job.occurrence, receipt.claim, now(), false); return false; }
      const scope = new URL(worker.registration.scope);
      // The locally scheduled title wins over a stale or untrusted payload.
      await worker.registration.showNotification(visible.title, { body: "Your task reminder", icon: new URL("icon.svg", scope).href,
        tag: `todo-push-${job.occurrence}`, renotify: false, data: { url: scope.href } });
      await store.finish(job.occurrence, receipt.claim, now(), true);
      return true;
    } catch (error) { await store.finish(job.occurrence, receipt.claim, now(), false); throw error; }
  }
  function endpoint(state) {
    const url = new URL(state.service), local = ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname);
    if (url.username || url.password || url.search || url.hash || !url.pathname.endsWith("/")
      || (url.protocol !== "https:" && !(local && url.protocol === "http:"))) throw new Error("Invalid reminder service address.");
    return new URL(`v1/installations/${state.id}`, url);
  }
  async function request(state, method, body, revision, fetcher) {
    const response = await fetcher(endpoint(state), { method, mode: "cors", credentials: "omit", redirect: "error", cache: "no-store",
      referrerPolicy: "no-referrer", signal: AbortSignal.timeout(15_000),
      headers: { authorization: `Bearer ${state.capability}`, ...(body ? { "content-type": "application/json", "if-match": `"${revision}"` } : {}) },
      body: body ? JSON.stringify(body) : undefined });
    if (!response.ok) throw Object.assign(new Error("Refresh background reminders in To Do."), { status: response.status });
    if (response.status === 204) return null;
    const length = Number(response.headers.get("content-length"));
    if (length > 4096) throw new Error("Invalid reminder service response.");
    const reader = response.body.getReader(); let text = "", bytes = 0; const decoder = new TextDecoder();
    for (;;) {
      const { done, value } = await reader.read(); if (done) break;
      bytes += value.length; if (bytes > 4096) { await reader.cancel(); throw new Error("Invalid reminder service response."); }
      text += decoder.decode(value, { stream: true });
    }
    text += decoder.decode(); const result = JSON.parse(text);
    if (!Number.isSafeInteger(result.revision) || result.revision < 0 || !Number.isSafeInteger(result.expiresAt)) throw new Error("Invalid reminder service response.");
    return result;
  }
  async function subscriptionChanged(worker, store, event, { fetcher = fetch, now = Date.now } = {}) {
    const initial = await store.read();
    if (!initial?.enabled) {
      await event.newSubscription?.unsubscribe(); return;
    }
    if (worker.Notification?.permission === "denied") {
      await store.update(state => state?.id === initial.id ? { ...state, enabled: false, plan: {}, workspace: null, needsUnsubscribe: false,
        pendingDeletes: [...(state.pendingDeletes || []), { id: state.id, capability: state.capability, service: state.service }],
        status: "Notification permission was removed. Background reminders are off." } : state);
      await event.newSubscription?.unsubscribe();
      try { await request(initial, "DELETE", null, null, fetcher); } catch {}
      return;
    }
    try {
      let subscription = event.newSubscription || await worker.registration.pushManager.getSubscription();
      if (!subscription) {
        const key = initial.config.publicKey;
        const bytes = Uint8Array.from(atob(key.replaceAll("-", "+").replaceAll("_", "/") + "="), character => character.charCodeAt(0));
        subscription = await worker.registration.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: bytes });
      }
      for (let attempt = 0; attempt < 3; attempt++) {
        let state = await store.read();
        if (!state?.enabled || state.id !== initial.id || !state.workspace) return;
        const remote = await request(state, "GET", null, null, fetcher);
        state = await store.read();
        if (!state?.enabled || state.id !== initial.id || !state.workspace) return;
        const jobs = Object.values(state.plan || {}).filter(job => job.expiresAt > now()).map(({ id, at, expiresAt, title }) => ({ id, at, expiresAt, title }));
        try {
          const result = await request(state, "PUT", { workspace: state.workspace, jobs, subscription: subscription.toJSON() }, remote.revision, fetcher);
          await store.update(current => current?.id === state.id ? { ...current, revision: result.revision, expiresAt: result.expiresAt,
            needsRefresh: false, syncedDigest: "", status: "The browser renewed its push subscription. Open To Do to refresh the active schedule." } : current);
          return;
        } catch (error) { if (error.status !== 409 || attempt === 2) throw error; }
      }
    } catch {
      await store.update(state => state?.id === initial.id ? { ...state, needsRefresh: true,
        status: "The browser's push subscription changed. Open To Do and choose Refresh background reminders." } : state);
    }
  }
  function install(worker, store) {
    worker.addEventListener("message", event => {
      if (event.data?.type === "TODO_PUSH_CHECK" && event.data.version === 1) event.ports[0]?.postMessage({ type: "TODO_PUSH_READY", version: 1 });
    });
    worker.addEventListener("push", event => {
      event.waitUntil((async () => {
        const text = event.data?.text(); if (!text || text.length > 8192) return;
        let payload; try { payload = JSON.parse(text); } catch { return; }
        await receive(worker, store, payload);
      })());
    });
    worker.addEventListener("pushsubscriptionchange", event => event.waitUntil(subscriptionChanged(worker, store, event)));
  }
  globalThis.TodoPushWorker = { install, receive, subscriptionChanged };
})();
