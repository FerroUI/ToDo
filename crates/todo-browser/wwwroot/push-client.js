import { randomToken, workspaceToken, buildPlan, wireJobs, reminderIdentity, digest } from "./push-plan.js";
import { serviceUrl, publicKey, serviceConfig, serviceRequest } from "./push-api.js";

const installationPath = state => `v1/installations/${state.id}`;
const sameDocument = (left, right) => left?.account === right?.account && left?.json === right?.json && !left?.blocked && !right?.blocked;

/** The durable document is authoritative, including changes from other tabs.
 * The private sidecar only stores this installation's delivery configuration. */
export class PushClient {
  constructor({ scope, store, readWorkspace, registration, permission, requestPermission, fetcher = fetch,
    lock = (_name, action) => action(), now = Date.now, onState = () => {}, supported = true, reason = "" }) {
    Object.assign(this, { scope, store, readWorkspace, registration, permission, requestPermission, fetcher, lock, now, onState, supported, reason });
    this.pending = Promise.resolve(); this.localPending = Promise.resolve(); this.timer = null; this.closed = false; this.importEpoch = 0;
  }
  request(base, path, options) { return serviceRequest(base, path, { ...options, fetcher: this.fetcher }); }
  exclusive(action) {
    const next = this.pending.then(() => this.lock(`ferroui.todo.push:${this.scope}`, action));
    this.pending = next.catch(() => {}); return next;
  }
  async publish(message) {
    const state = this.supported ? await this.store.read() : null;
    const enabled = !!state?.enabled;
    this.lastPresentation = { available: true, supported: this.supported, enabled, service_url: state?.service || "", public_key: state?.config?.publicKey || "",
      status: message || (!this.supported ? this.reason : state?.status || "A user-configured reminder service is required for delivery after this page closes.") };
    this.onState(this.lastPresentation);
    return state;
  }
  async start() {
    await this.publish();
    if (!this.supported) return;
    await this.store.prune(this.now());
    await this.sync().catch(error => this.failed(error));
  }
  async failed(error) {
    const message = String(error?.message || error).slice(0, 500);
    try { await this.store.update(state => state ? { ...state, status: message } : null); } catch {}
    try { await this.publish(message); }
    catch {
      // Error reporting cannot depend on the storage capability that failed.
      this.onState({ available: true, supported: this.supported, enabled: false, service_url: "", public_key: "", ...this.lastPresentation, status: message });
    }
  }
  changed() {
    if (!this.supported || this.closed) return;
    // Cancellation reaches the worker's local gate before the debounced upload.
    void this.localPlan().catch(error => this.failed(error));
    clearTimeout(this.timer);
    this.timer = setTimeout(() => { this.timer = null; void this.sync().catch(error => this.failed(error)); }, 400);
  }
  imported() {
    this.importEpoch++;
    if (!this.supported) return;
    const localEpoch = randomToken();
    void this.store.update(state => state ? { ...state, localEpoch, plan: {}, workspace: null, planDigest: "" } : null)
      .then(() => this.changed()).catch(error => this.failed(error));
  }
  workspace(state, document) {
    return workspaceToken(state.seed, document.account === null ? { local: state.localEpoch || "" } : document.account);
  }
  localPlan() {
    const next = this.localPending.then(async () => {
      for (let attempt = 0; attempt < 4; attempt++) {
        const importEpoch = this.importEpoch;
        const state = await this.store.read();
        if (!state?.enabled) return state;
        const document = this.readWorkspace();
        if (document.blocked) {
          return this.store.update(current => current?.id === state.id ? { ...current, workspace: null, plan: {}, planDigest: "", status: "Account switch in progress; reminder delivery is paused." } : current);
        }
        const workspace = await this.workspace(state, document), parsed = JSON.parse(document.json);
        const { plan, omitted } = await buildPlan(parsed, workspace, state.config, this.now());
        const planDigest = await digest(JSON.stringify([workspace, wireJobs(plan)]));
        if (!sameDocument(document, this.readWorkspace())) continue;
        let accepted = false;
        const result = await this.store.update(current => {
          // The IDB callback can run after the hashing awaits and after another
          // tab or an import changed the private generation. Check both durable
          // document and generation at the actual write boundary.
          if (current?.id !== state.id || !current.enabled || current.seed !== state.seed
            || (current.localEpoch || "") !== (state.localEpoch || "") || this.importEpoch !== importEpoch
            || !sameDocument(document, this.readWorkspace())) return current;
          accepted = true;
          return { ...current, workspace, plan, planDigest, omitted };
        });
        if (!accepted) continue;
        await this.store.prune(this.now(), { workspace, reminders: Object.fromEntries(parsed.tasks.filter(task => task.reminder).map(task => [String(task.id), task.reminder])) });
        return result;
      }
      throw new Error("Tasks are changing in another tab. Reminder scheduling will retry after the next save.");
    });
    this.localPending = next.catch(() => {}); return next;
  }
  async cleanPending(state) {
    if (!state?.pendingDeletes?.length) return state;
    const removed = new Set();
    for (const pending of state.pendingDeletes) {
      try { await this.request(pending.service, installationPath(pending), { method: "DELETE", capability: pending.capability }); removed.add(`${pending.service}:${pending.id}`); }
      catch (error) { if ([401, 404, 410].includes(error.status)) removed.add(`${pending.service}:${pending.id}`); }
    }
    return this.store.update(current => current?.id === state.id ? { ...current, pendingDeletes: (current.pendingDeletes || []).filter(item => !removed.has(`${item.service}:${item.id}`)) } : current);
  }
  enable({ service_url, public_key, enrollment_key }) {
    if (!this.supported) return Promise.reject(new Error(this.reason));
    const service = serviceUrl(service_url), key = String(public_key || "").trim(); publicKey(key);
    if (!/^[A-Za-z0-9_-]{43,128}$/.test(enrollment_key || "")) return Promise.reject(new Error("Enter the enrollment key supplied by your reminder service administrator."));
    // Start the permission request within the explicit button gesture.
    const granted = this.permission() === "granted" ? Promise.resolve("granted") : this.requestPermission();
    return this.exclusive(async () => {
      if (await granted !== "granted") throw new Error("Allow notifications in this site's browser settings to enable background reminders.");
      let previous = await this.cleanPending(await this.store.read());
      if (previous?.enabled) throw new Error("Background reminders are already enabled. Disable them before changing services.");
      const config = serviceConfig(await this.request(service, "v1/config"), key);
      const registration = await this.registration();
      let subscription = await registration.pushManager.getSubscription();
      if (subscription) {
        const actual = subscription.options?.applicationServerKey;
        if (!actual || !sameBytes(actual, publicKey(key))) { await subscription.unsubscribe(); subscription = null; }
      }
      subscription ||= await registration.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: publicKey(key) });
      const retry = previous?.enrolling && previous.service === service && previous.config.publicKey === key;
      const pendingDeletes = [...(previous?.pendingDeletes || [])];
      if (previous?.enrolling && !retry) pendingDeletes.push({ id: previous.id, capability: previous.capability, service: previous.service });
      if (pendingDeletes.length > 8) throw new Error("Previous reminder services still need cleanup. Reconnect and disable them before enrolling another service.");
      const state = { id: retry ? previous.id : randomToken(16), capability: retry ? previous.capability : randomToken(),
        seed: retry ? previous.seed : randomToken(), service, config, enabled: false, enrolling: true, revision: 0,
        plan: {}, workspace: null, expiresAt: 0, pendingDeletes, status: "Enrolling this browser…" };
      await this.store.update(() => state);
      if (!retry) await this.store.resetReceipts();
      const result = await this.request(service, "v1/installations", { method: "POST", enrollment: enrollment_key,
        body: { id: state.id, capability: state.capability, scope: this.scope, subscription: subscription.toJSON() } });
      await this.store.update(current => current?.id === state.id ? { ...current, enabled: true, enrolling: false,
        revision: result.revision, expiresAt: result.expiresAt, status: "Preparing the active workspace's reminders…" } : current);
      await this.syncUnlocked(true);
    }).catch(async error => { await this.failed(error); throw error; });
  }
  sync(force = false) { return this.exclusive(() => this.syncUnlocked(force)); }
  async syncUnlocked(force = false, replacement = null) {
    let state = await this.cleanPending(await this.store.read());
    if (!state?.enabled) {
      if (state?.needsUnsubscribe) await this.disableUnlocked();
      else await this.publish();
      return;
    }
    if (this.permission() !== "granted") { await this.disableUnlocked("Notification permission was removed. Background reminders are off."); return; }
    if (!replacement) {
      const registration = await this.registration(), subscription = await registration.pushManager.getSubscription();
      if (state.needsRefresh || !subscription || !sameBytes(subscription.options?.applicationServerKey || [], publicKey(state.config.publicKey))) {
        throw new Error("The browser's push subscription changed. Choose Refresh background reminders.");
      }
    }
    for (let attempt = 0; attempt < 3; attempt++) {
      state = await this.localPlan();
      if (!state?.enabled || !state.workspace) { await this.publish(); return; }
      if (!force && !state.needsRefresh && !replacement && state.syncedDigest === state.planDigest && state.expiresAt > this.now() + 7 * 86_400_000) {
        await this.publish(); return;
      }
      const installation = state.id;
      const remote = await this.request(state.service, installationPath(state), { capability: state.capability });
      if (!remote.subscriptionActive && !replacement) throw new Error("The push subscription expired. Choose Refresh background reminders.");
      // GET may have crossed an account switch, a save, or another tab's import.
      state = await this.localPlan();
      if (!state?.enabled || !state.workspace) return;
      if (state.id !== installation) continue;
      const body = { workspace: state.workspace, jobs: wireJobs(state.plan), ...(replacement ? { subscription: replacement.toJSON() } : {}) };
      try {
        const result = await this.request(state.service, installationPath(state), { method: "PUT", capability: state.capability, revision: remote.revision, body });
        const status = `${Object.keys(state.plan).length} reminder${Object.keys(state.plan).length === 1 ? "" : "s"} scheduled from the active workspace. `
          + `Refresh by ${new Date(result.expiresAt).toLocaleDateString()} to renew this installation.`
          + (state.omitted ? ` ${state.omitted} reminder${state.omitted === 1 ? " is" : "s are"} outside the service's time or count limits.` : "");
        await this.store.update(current => current?.id === state.id && current.enabled ? { ...current, revision: result.revision,
          expiresAt: result.expiresAt, syncedDigest: state.planDigest, needsRefresh: false, status } : current);
        await this.publish(); return;
      } catch (error) {
        if (error.status !== 409 || attempt === 2) throw error;
        // Never retry a stale body. Rebuild from the current durable workspace.
      }
    }
  }
  refresh() {
    if (!this.supported) return Promise.reject(new Error(this.reason));
    return this.exclusive(async () => {
      const state = await this.store.read();
      if (!state?.enabled) { await this.cleanPending(state); await this.publish(); return; }
      if (this.permission() !== "granted") { await this.disableUnlocked("Allow notifications and enroll again to restore background reminders."); return; }
      const remote = await this.request(state.service, installationPath(state), { capability: state.capability });
      const registration = await this.registration();
      let subscription = await registration.pushManager.getSubscription();
      if (subscription && (!remote.subscriptionActive || !sameBytes(subscription.options?.applicationServerKey || [], publicKey(state.config.publicKey)))) {
        await subscription.unsubscribe(); subscription = null;
      }
      subscription ||= await registration.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: publicKey(state.config.publicKey) });
      await this.syncUnlocked(true, subscription);
    }).catch(async error => { await this.failed(error); throw error; });
  }
  disable() { return this.exclusive(() => this.disableUnlocked()); }
  async disableUnlocked(message = "Background reminders are off.") {
    const state = await this.store.update(current => current ? { ...current, enabled: false, enrolling: false, plan: {}, workspace: null,
      status: message, pendingDeletes: [...(current.pendingDeletes || []), ...(!current.pendingDeletes?.some(item => item.id === current.id)
        ? [{ id: current.id, capability: current.capability, service: current.service }] : [])] } : null);
    let unsubscribeFailed = false;
    try {
      const registration = await this.registration();
      const subscription = await registration.pushManager.getSubscription();
      if (subscription) unsubscribeFailed = !await subscription.unsubscribe();
    } catch { unsubscribeFailed = true; }
    const cleaned = await this.cleanPending(state);
    const status = message + (unsubscribeFailed ? " Browser subscription cleanup will retry when this app reopens." : "")
      + (cleaned?.pendingDeletes?.length ? " Service cleanup is pending and will retry when connected." : "");
    await this.store.update(current => current?.id === state?.id ? { ...current, needsUnsubscribe: unsubscribeFailed, status } : current);
    await this.publish(status);
  }
  async foreground(action, show) {
    const document = this.readWorkspace();
    if (document.blocked) return true;
    const epoch = this.importEpoch;
    const currentTask = () => {
      const current = this.readWorkspace();
      if (current.blocked || current.account !== document.account || this.importEpoch !== epoch) return null;
      const task = JSON.parse(current.json).tasks.find(task => String(task.id) === String(action.task_id));
      return task && !task.completed && task.reminder?.date === action.date && task.reminder?.minutes_after_midnight === action.minutes ? task : null;
    };
    const ordinary = async () => { if (currentTask()) await show({ shouldShow: () => !!currentTask(), getTitle: () => currentTask()?.title || action.title }); return true; };
    if (!this.supported) return ordinary();
    let displayStarted = false;
    try {
      const state = await this.localPlan();
      if (!currentTask()) return true;
      if (!state?.enabled) return ordinary();
      const workspace = await this.workspace(state, document);
      const identity = await reminderIdentity(workspace, action.task_id, action.date, action.minutes);
      if (!identity || !currentTask()) return true;
      const now = this.now();
      const receipt = await this.store.claim({ installation: state.id, workspace, ...identity, now, until: identity.at + state.config.expiryMs, requirePlan: false });
      if (!receipt) {
        const current = await this.store.read();
        if (!current?.enabled && currentTask()) return ordinary();
        return true;
      }
      try {
        if (!currentTask()) { await this.store.finish(identity.occurrence, receipt.claim, this.now(), false); return true; }
        displayStarted = true;
        const displayed = await show({ tag: `todo-push-${identity.occurrence}`, shouldShow: () => !!currentTask(), getTitle: () => currentTask()?.title || action.title });
        await this.store.finish(identity.occurrence, receipt.claim, this.now(), displayed !== false);
      }
      catch (error) { await this.store.finish(identity.occurrence, receipt.claim, this.now(), false); throw error; }
      return true;
    } catch (error) {
      await this.failed(error);
      if (displayStarted) return true;
      return ordinary();
    }
  }
  close() { this.closed = true; clearTimeout(this.timer); }
}

function sameBytes(left, right) {
  const bytes = new Uint8Array(left); return bytes.length === right.length && bytes.every((value, index) => value === right[index]);
}
