import * as auth from "./cloud/auth.js";
import { graph, clearCloudCache } from "./cloud/client.js";
import { recoverCommit, forgetDocumentMappings } from "./cloud/metadata.js";
import { synchronize } from "./cloud/sync.js";
import { synchronizePlanner, forgetPlannerMappings } from "./cloud/planner.js";
import { serializeMicrosoft } from "./cloud/operations.js";
import { automaticSync, requiresSyncReview } from "./cloud/automatic.js";
import { connectedActions, connectedAppRequest, connectedPreferencesChanged, resetConnectedApps, restoreConnectedApps, synchronizeConnectedApps } from "./cloud/connected.js";

let runtime, startupError, restored, configuration;
let preferences = { auto_sync: true, planner_enabled: false };
let plannerPaused = false;
let ownCommit = false;
let barriers = 0;
const demo = () => new URLSearchParams(location.search).get("demo") === "1";
const operations = serializeMicrosoft(performRequest);
const today = () => {
  const date = new Date();
  return `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2,"0")}-${String(date.getDate()).padStart(2,"0")}`;
};
const automatic = automaticSync({
  ready: () => !!runtime && !!auth.account() && !auth.sessionStatus().needsInteraction
    && barriers === 0 && preferences.auto_sync !== false && !demo() && !document.hidden && navigator.onLine !== false,
  run: () => operations({ kind: "syncMicrosoft", automatic: true }, today()),
  report: error => {
    runtime.cloudState(!!auth.account(), auth.sessionStatus().needsInteraction ? "Sign-in required · Reconnect your Microsoft account" : "Microsoft sync needs attention");
    runtime.hostError(error.message);
  },
});

export async function prepareMicrosoft() {
  try {
    recoverCommit();
    configuration = await auth.configuration();
    if (!demo()) restored = await auth.restore();
  } catch (error) { startupError = error; restored = auth.account(); }
}

function capabilities() {
  runtime?.cloudCapabilities(true, !!configuration?.configured, auth.isWorkAccount());
}

export async function attachMicrosoft(value) {
  runtime = value;
  capabilities();
  microsoftDocumentChanged(runtime.exportData());
  if (startupError?.optionalConsent) {
    for (const scope of startupError.optionalScopes || []) {
      if (scope.endsWith("MailboxSettings.Read")) runtime.setIntegrationPreference("outlook_categories_enabled", false);
      if (scope.endsWith("Files.ReadWrite.AppFolder")) runtime.setIntegrationPreference("presentation_sync_enabled", false);
    }
  }
  if (startupError) runtime.hostError(startupError.message);
  document.addEventListener("visibilitychange", () => document.hidden ? automatic.pause() : automatic.resume());
  window.addEventListener("online", () => automatic.resume());
  window.addEventListener("offline", () => automatic.pause());
  window.addEventListener("pagehide", () => automatic.pause());
  if (restored) await operations({ kind: "restoreMicrosoft" });
  automatic.start();
}

export function microsoftDocumentChanged(contents) {
  try {
    const next = JSON.parse(contents).settings || {};
    if (preferences.planner_enabled !== next.planner_enabled) plannerPaused = false;
    if (preferences.outlook_categories_enabled !== next.outlook_categories_enabled
      || preferences.presentation_sync_enabled !== next.presentation_sync_enabled) connectedPreferencesChanged();
    preferences = next;
    if (!ownCommit) automatic.changed();
  } catch { /* Rust reports invalid documents at the persistence boundary. */ }
}

export function microsoftRequest(action, date = today()) {
  if (action.kind === "syncMicrosoft") automatic.retry();
  if (["connectMicrosoft","reconnectMicrosoft","disconnectMicrosoft","signOutMicrosoft", ...connectedActions].includes(action.kind)) {
    return barrier(() => operations(action, date));
  }
  return operations(action, date);
}

function barrier(perform) {
  barriers++; automatic.pause();
  // Enter the operation queue synchronously so later requests cannot overtake us.
  let operation;
  try { operation = perform(); } catch (error) { operation = Promise.reject(error); }
  return Promise.resolve(operation).finally(() => {
    if (--barriers === 0) automatic.resume();
  });
}

// File restoration and account exports share the same queue as synchronization.
export function microsoftExclusive(perform, { restore = false } = {}) {
  return barrier(() => operations({ kind: "localDocumentOperation", perform, restore }));
}

export function microsoftAccountId() { return auth.accountKey(); }

async function performRequest(action, date) {
  if (action.kind === "localDocumentOperation") {
    const perform = async lock => {
      if (!lock) throw new Error("Another tab is synchronizing. Finish that operation before restoring or exporting this document.");
      const result = await action.perform();
      if (result && action.restore) {
        automatic.stop();
        try { await auth.disconnect(); }
        finally {
          forgetDocumentMappings(); forgetPlannerMappings(); clearCloudCache();
          resetConnectedApps(runtime, { restore: true });
          runtime.cloudState(false, "Local account");
          configuration = await auth.configuration().catch(() => null); capabilities();
          runtime.hostStatus("Restored as a local copy. Microsoft sync is paused.");
        }
      }
      return result;
    };
    return navigator.locks ? navigator.locks.request("ferroui-todo-microsoft-sync", { ifAvailable: true }, perform) : perform(true);
  }
  if (action.kind === "restoreMicrosoft") {
    try {
      const user = await graph("me?$select=id,displayName,userPrincipalName");
      runtime.cloudState(true, user.displayName || user.userPrincipalName || restored.username);
      restoreConnectedApps(runtime, auth.accountKey(), preferences);
    } catch (error) {
      runtime.cloudState(!!auth.account(), "Sign-in required · Reconnect your Microsoft account");
      runtime.hostError(error.message);
    }
    capabilities(); return;
  }
  if (demo()) throw new Error("Open the normal app to connect an account; demo tasks are kept separate.");
  if (["connectMicrosoft", "reconnectMicrosoft", "signOutMicrosoft"].includes(action.kind)) runtime.flushEdits();
  if (connectedActions.includes(action.kind)) return connectedAppRequest(runtime, action.kind);
  if (action.kind === "connectMicrosoft") return auth.connect(action.client_id);
  if (action.kind === "reconnectMicrosoft") return auth.reconnect();
  if (["disconnectMicrosoft", "signOutMicrosoft"].includes(action.kind)) {
    try {
      if (action.kind === "signOutMicrosoft") await auth.logout();
      else await auth.disconnect();
    }
    finally {
      clearCloudCache(); runtime.cloudState(false, "Local account");
      resetConnectedApps(runtime);
      configuration = await auth.configuration().catch(() => null); capabilities();
    }
    return;
  }
  if (action.kind !== "syncMicrosoft") throw new Error("Unknown Microsoft account action.");
  const account = auth.account();
  if (!account) throw new Error("Connect your Microsoft account first.");
  const run = async lock => {
    if (!lock) {
      if (action.automatic) return;
      throw new Error("Another tab is synchronizing. Try again when it finishes.");
    }
    if (!action.automatic) runtime.cloudState(true, "Syncing…");
    try {
      ownCommit = true;
      let changed = await synchronize(runtime, auth.accountKey(), date);
      if (preferences.planner_enabled && auth.isWorkAccount() && (!plannerPaused || !action.automatic)) {
        try {
          await synchronizePlanner(runtime, auth.accountKey(), date, { force: !action.automatic });
          plannerPaused = false;
        } catch (error) {
          plannerPaused = requiresSyncReview(error);
          if (!plannerPaused || !action.automatic) throw error;
          runtime.hostError(`Planner sync needs attention: ${error.message}`);
        }
      }
      const connected = await synchronizeConnectedApps(runtime, auth.accountKey(), preferences, { force: !action.automatic });
      changed ||= connected.changed;
      if (changed || !action.automatic || connected.issues.length) runtime.cloudState(true,
        `${account.username || "Microsoft account"} · ${connected.issues.length ? "Tasks synced · Connected app needs attention" : "Synced"}`);
    } catch (error) {
      if (!action.automatic) runtime.cloudState(true, `${account.username || "Microsoft account"} · Sync needs attention`);
      throw error;
    } finally { ownCommit = false; }
  };
  if (navigator.locks) return navigator.locks.request("ferroui-todo-microsoft-sync", { ifAvailable: true }, run);
  throw new Error("This browser cannot safely coordinate cloud synchronization. Use a current browser with Web Locks support.");
}
