import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";

const modules = {
  "auth.js": `
    const fixture = () => globalThis.microsoftHostFixture;
    export const configuration = async () => ({ configured: true });
    export const restore = async () => fixture().account;
    export const account = () => fixture().account;
    export const accountKey = () => fixture().account?.homeAccountId || null;
    export const serviceEnvironment = () => ({ id: "global", todoUrl: "https://to-do.office.com/tasks/" });
    export const isWorkAccount = () => !!fixture().account;
    export const sessionStatus = () => ({ needsInteraction: false });
    export const connect = async () => fixture().events.push("connect");
    export const reconnect = async () => fixture().events.push("reconnect");
    export const logout = async () => {
      const current = fixture(); current.account = null;
      if (current.logoutError) throw current.logoutError;
    };
    export const disconnect = async () => {
      const current = fixture(); current.events.push("disconnect"); current.account = null;
      if (current.disconnectError) throw current.disconnectError;
    };`,
  "client.js": `
    export const graph = async () => ({ displayName: "Example" });
    export const readCloud = async () => [];
    export const clearCloudCache = () => globalThis.microsoftHostFixture.events.push("clear-cache");`,
  "metadata.js": `
    export const recoverCommit = () => {};
    export const forgetDocumentMappings = () => globalThis.microsoftHostFixture.events.push("forget-todo");`,
  "sync.js": `
    export const synchronize = (...args) => globalThis.microsoftHostFixture.sync(...args);`,
  "planner.js": `
    export const synchronizePlanner = (...args) => globalThis.microsoftHostFixture.planner(...args);
    export const forgetPlannerMappings = () => globalThis.microsoftHostFixture.events.push("forget-planner");`,
  "connected.js": `
    export const connectedActions = ['enableOutlookCategories','refreshOutlookCategories','disableOutlookCategories','enablePresentationSync','disablePresentationSync'];
    export const connectedAppRequest = async (_runtime,kind) => { globalThis.microsoftHostFixture.events.push(kind); return true; };
    export const connectedPreferencesChanged = () => {};
    export const resetConnectedApps = () => {};
    export const restoreConnectedApps = () => {};
    export const synchronizeConnectedApps = async () => ({changed:false,issues:[]});`,
  "attachment-content.js": `
    export const materializeCloudAttachment = async () => null;
    export const openCloudAttachment = async () => {};
    export const cloudAttachmentKey = () => "graph:fixture";`,
  "attachments": `
    const fixture = () => globalThis.microsoftHostFixture;
    export const chooseFile = async () => fixture().selectedFile ? [fixture().selectedFile] : [];
    export const readAttachment = async key => fixture().files.get(key);
    export const storeAttachment = async (key, file) => { fixture().files.set(key, file); };
    export const deleteAttachment = async key => { fixture().removed.push(key); fixture().files.delete(key); };
    export const saveAttachment = async () => "unused";
    export const openAttachment = async () => {};
    export const downloadBlob = () => {};`,
  "graph-host": `
    const api = () => globalThis.microsoftHostFixture.api;
    export const microsoftRequest = (...args) => api().microsoftRequest(...args);
    export const microsoftDocumentChanged = (...args) => api().microsoftDocumentChanged(...args);
    export const microsoftExclusive = (...args) => api().microsoftExclusive(...args);
    export const microsoftAccountId = () => api().microsoftAccountId();`,
};
registerHooks({
  resolve(specifier, context, next) {
    const result = next(specifier, context);
    if (result.url.endsWith("/wwwroot/attachments.js")) return { url: "fixture:microsoft-host/attachments", shortCircuit: true };
    if (result.url.endsWith("/wwwroot/graph.js") && context.parentURL?.includes("/wwwroot/host.js?")) {
      return { url: "fixture:microsoft-host/graph-host", shortCircuit: true };
    }
    const name = result.url.split("/cloud/")[1];
    return Object.hasOwn(modules, name) ? { url: `fixture:microsoft-host/${name}`, shortCircuit: true } : result;
  },
  load(url, context, next) {
    if (!url.startsWith("fixture:microsoft-host/")) return next(url, context);
    return { format: "module", shortCircuit: true, source: modules[url.split("/").at(-1)] };
  },
});

const settle = () => new Promise(resolve => setImmediate(resolve));
let instance = 0;

async function harness(t) {
  const saved = new Map();
  const replace = (name, value) => {
    saved.set(name, Object.getOwnPropertyDescriptor(globalThis, name));
    Object.defineProperty(globalThis, name, { value, configurable: true, writable: true });
  };
  const timers = new Map(); let nextTimer = 0;
  const h = { timers, events: [], calls: [], statuses: [], errors: [], files: new Map(), removed: [], lockAvailable: true,
    account: { homeAccountId: "account-one", username: "example@example.test" },
    settings: { auto_sync: true, planner_enabled: true },
    async sync(...args) {
      h.events.push("todo");
      return h.onSync ? h.onSync(...args) : false;
    },
    async planner(...args) {
      h.calls.push(args[3]); h.events.push("planner");
      if (h.onPlanner) return h.onPlanner(...args);
    },
    async fire() {
      const entry = [...timers][0];
      assert(entry, "an automatic sync is scheduled");
      timers.delete(entry[0]); entry[1].callback(); await settle();
    },
  };
  replace("microsoftHostFixture", h);
  replace("location", { search: "" });
  replace("document", Object.assign(new EventTarget(), { hidden: false }));
  replace("window", new EventTarget());
  replace("navigator", { onLine: true, locks: {
    request: async (_name, _options, callback) => callback(h.lockAvailable ? {} : null),
  } });
  replace("setTimeout", (callback, delay) => { timers.set(++nextTimer, { callback, delay }); return nextTimer; });
  replace("clearTimeout", id => timers.delete(id));
  t.after(() => {
    timers.clear();
    for (const [name, descriptor] of saved) {
      if (descriptor) Object.defineProperty(globalThis, name, descriptor);
      else delete globalThis[name];
    }
  });
  h.document = { schema_version: 1, settings: h.settings, lists: [{ id: 1, name: "Tasks" }], tasks: [] };
  h.runtime = {
    exportData: () => JSON.stringify(h.document),
    validateImportData: json => { assert.equal(JSON.parse(json).schema_version, 1); return json; },
    importData: json => {
      if (h.importError) throw h.importError;
      h.document = JSON.parse(json); h.settings = h.document.settings;
      h.api.microsoftDocumentChanged(json); h.completion?.resolve();
    },
    cloudCapabilities() {}, flushEdits() {},
    cloudState: (...args) => h.statuses.push(args),
    hostError: message => { h.errors.push(message); h.completion?.reject(new Error(message)); },
    hostStatus: message => h.events.push(message),
  };
  h.api = await import(`../crates/todo-browser/wwwroot/graph.js?host-test=${++instance}`);
  await h.api.prepareMicrosoft(); await h.api.attachMicrosoft(h.runtime);
  return h;
}

test("manual refresh retries paused Planner after an active automatic sync and surfaces its failure", async t => {
  const h = await harness(t);
  h.onPlanner = async () => { throw Object.assign(new Error("Planner is unavailable"), { status: 403 }); };
  await h.fire();
  assert.deepEqual(h.calls, [{ force: false }]);
  assert.match(h.errors.at(-1), /Planner sync needs attention/);
  const gate = Promise.withResolvers(); h.onSync = () => gate.promise;
  await h.fire();
  const manual = h.api.microsoftRequest({ kind: "syncMicrosoft" });
  const rejected = assert.rejects(manual, /Planner is unavailable/);
  gate.resolve(false); await rejected; await settle();
  assert.equal(h.calls.at(-1).force, true, "manual retry bypasses Planner's pause and automatic throttle");
  assert.equal(h.calls.length, 2);
});

test("transient Planner failures back off and recover without requiring a manual refresh", async t => {
  const h = await harness(t);
  h.onPlanner = async () => { throw Object.assign(new Error("Planner service unavailable"), { status: 503 }); };
  await h.fire();
  assert.equal(h.calls.length, 1); assert.equal(h.timers.size, 1);
  h.onPlanner = undefined;
  await h.fire();
  assert.equal(h.calls.length, 2, "a temporary service outage must not permanently pause Planner");
  assert.equal(h.calls.at(-1).force, false);
});

test("successful restoration clears old mappings even if MSAL token-cache cleanup fails", async t => {
  const h = await harness(t);
  h.disconnectError = new Error("Token-cache cleanup failed");
  await assert.rejects(h.api.microsoftExclusive(async () => {
    h.settings.auto_sync = false;
    h.api.microsoftDocumentChanged(h.runtime.exportData());
    return { tasks: 1 };
  }, { restore: true }), /Token-cache cleanup failed/);
  assert.equal(h.account, null);
  assert(h.events.includes("forget-todo"), "restored data must not retain destructive To Do baselines");
  assert(h.events.includes("forget-planner"));
  assert(h.events.includes("clear-cache"));
  assert.equal(h.statuses.at(-1)[0], false);
  assert.equal(h.timers.size, 0);
});

test("a rejected Microsoft sign-out redirect still clears local connected status and cached account data", async t => {
  const h = await harness(t);
  h.logoutError = new Error("Sign-out redirect failed");
  await assert.rejects(h.api.microsoftRequest({ kind: "signOutMicrosoft" }), /Sign-out redirect failed/);
  assert.equal(h.account, null);
  assert.equal(h.statuses.at(-1)[0], false);
  assert(h.events.includes("clear-cache"));
  assert.equal(h.timers.size, 0);
});

test("a competing tab's lock defers local operations and resumes automatic synchronization", async t => {
  const h = await harness(t); h.lockAvailable = false;
  let performed = false;
  await assert.rejects(h.api.microsoftExclusive(async () => { performed = true; }), /Another tab is synchronizing/);
  assert.equal(performed, false);
  assert.equal(h.timers.size, 1, "a rejected export must not silently disable automatic sync");
  h.lockAvailable = true; await h.fire();
  assert.deepEqual(h.calls, [{ force: false }]);
});

test("a restore barrier stays suspended while an earlier automatic sync completes", async t => {
  const h = await harness(t);
  const syncGate = Promise.withResolvers(), restoreGate = Promise.withResolvers();
  h.onSync = () => syncGate.promise;
  await h.fire();
  let restoring = false;
  const restore = h.api.microsoftExclusive(async () => {
    restoring = true; await restoreGate.promise;
    h.settings.auto_sync = false;
    h.api.microsoftDocumentChanged(h.runtime.exportData());
    return { tasks: 2 };
  }, { restore: true });
  syncGate.resolve(false); await settle();
  assert.equal(restoring, true);
  assert.equal(h.timers.size, 0, "finishing the earlier sync must not schedule work across a suspended restore");
  restoreGate.resolve(); await restore;
  assert.equal(h.account, null);
  assert.equal(h.timers.size, 0);
});

test("turning automatic sync off cancels scheduled work and turning it on debounces once", async t => {
  const h = await harness(t);
  h.settings.auto_sync = false; h.api.microsoftDocumentChanged(h.runtime.exportData());
  assert.equal(h.timers.size, 0);
  h.settings.auto_sync = true; h.api.microsoftDocumentChanged(h.runtime.exportData());
  assert.equal(h.timers.size, 1);
  assert.equal([...h.timers.values()][0].delay, 900);
  await h.fire(); assert.deepEqual(h.calls, [{ force: false }]);
});

test("an account barrier retains request order when a manual sync follows immediately", async t => {
  const h = await harness(t);
  const disconnect = h.api.microsoftRequest({ kind: "disconnectMicrosoft" });
  const later = h.api.microsoftRequest({ kind: "syncMicrosoft" });
  await assert.rejects(later, /Connect your Microsoft account first/);
  await disconnect;
  assert.equal(h.events.includes("todo"), false, "a later request cannot synchronize before an earlier disconnect");
});

async function hostArchive(h, kind) {
  const bytes = new TextEncoder().encode("plan");
  const sha256 = [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))].map(value => value.toString(16).padStart(2, "0")).join("");
  const file = { key: "backup:1", name: "plan.txt", type: "text/plain", size: bytes.length, base64: "cGxhbg==", sha256 };
  if (kind === "importMicrosoft") return {
    kind: "ferroui.microsoft-todo", version: 1, exportedAt: "2026-11-22T15:00:00Z",
    lists: [{ id: "list-one", displayName: "Tasks", wellknownListName: "defaultList", tasks: [{ id: "task-one", title: "With file",
      checklistItems: [], linkedResources: [], attachments: [{ id: "file-one", name: file.name, contentType: file.type, size: file.size }] }] }],
    files: [{ ...file, source: { listId: "list-one", taskId: "task-one", attachmentId: "file-one" } }],
  };
  return { kind: "ferroui.todo.backup", version: 1, document: { ...h.document, tasks: [{ id: 2, list_id: 1, title: "With file",
    attachments: [{ id: 3, storage_key: file.key, name: file.name, mime_type: file.type, size: file.size }] }] }, files: [file] };
}

async function importThroughHost(h, kind) {
  const archive = await hostArchive(h, kind);
  h.selectedFile = new File([JSON.stringify(archive)], "backup.json", { type: "application/json" });
  h.completion = Promise.withResolvers();
  const host = await import(`../crates/todo-browser/wwwroot/host.js?host-test=${++instance}`);
  host.attachRuntime(h.runtime); host.request(JSON.stringify({ kind }));
  return h.completion.promise;
}

for (const kind of ["import", "importMicrosoft"]) {
  test(`${kind} host action restores file contents through the attachment backend`, async t => {
    const h = await harness(t);
    await importThroughHost(h, kind); await settle();
    assert.equal(h.files.size, 1);
    const attachment = h.document.tasks[0].attachments[0];
    assert.notEqual(attachment.storage_key, "backup:1");
    assert.equal(await h.files.get(attachment.storage_key).text(), "plan");
    assert.equal(h.settings.auto_sync, false); assert.equal(h.account, null);
  });
  test(`${kind} host action rolls back file writes if document replacement fails`, async t => {
    const h = await harness(t); h.importError = new Error("Document replacement failed");
    await assert.rejects(importThroughHost(h, kind), /Document replacement failed/); await settle();
    assert.equal(h.files.size, 0); assert.equal(h.removed.length, 1);
    assert.equal(h.document.tasks.length, 0); assert.notEqual(h.account, null);
    assert.equal(h.events.includes("forget-todo"), false, "a failed restore keeps the existing account mapping");
  });
}
