import { register } from "node:module";
import { webcrypto } from "node:crypto";
import { installStorage, atomicWrite, readBounded } from "./storage.mjs";
import { receive, send, ui } from "./bridge.mjs";
import { runtime, beginImport, endImport } from "./runtime.mjs";
import * as files from "./files.mjs";
import { NativeDOMParser } from "./html.mjs";

if (Number(process.versions.node.split(".")[0]) < 22) throw new Error("Native Microsoft services require Node.js 22 or newer.");
const parentPid = process.ppid;
// Awaited sign-in/network work must not keep session tokens alive after an
// abnormal host exit. Idle workers also exit immediately when stdin closes.
setInterval(() => {
  try { if (process.ppid !== parentPid) process.exit(0); process.kill(parentPid, 0); }
  catch (error) { if (error?.code === "ESRCH") process.exit(0); }
}, 1000).unref();
register("./loader.mjs", import.meta.url);
globalThis.location = new URL("https://ferroui.com/ToDo/");
globalThis.DOMParser = NativeDOMParser;
// Node>=22 provides WebCrypto. Keep the worker independent of runtime flags.
if (!globalThis.crypto) Object.defineProperty(globalThis, "crypto", { value: webcrypto });
const initial = receive();
if (initial?.type !== "initialize" || typeof initial.directory !== "string") throw new Error("Invalid native initialization.");
installStorage(initial.directory);
const auth = await import("./auth.mjs");
const client = await import("../web/cloud/client.js");
const metadata = await import("../web/cloud/metadata.js");
const { synchronize } = await import("../web/cloud/sync.js");
const { synchronizePlanner, forgetPlannerMappings } = await import("../web/cloud/planner.js");
const { exportPortable, restorePortable } = await import("../web/backup/archive.js");
const { exportMicrosoftArchive, restoreMicrosoftArchive } = await import("../web/cloud/archive.js");
const { materializeCloudAttachment } = await import("../web/cloud/attachment-content.js");
const { requiresSyncReview } = await import("../web/cloud/automatic.js");
const connected = await import("../web/cloud/connected.js");
let plannerPaused = false;
let lastPreferences = "";

async function state() {
  const configuration = await auth.configuration();
  runtime.cloudCapabilities(true, configuration.configured, auth.isWorkAccount());
  runtime.cloudHint("Native Microsoft services use a private Node.js worker. Sign-in stays in your system browser; tokens stay in this app session.");
}

async function disconnect(restore = false) {
  await auth.disconnect(); client.clearCloudCache(); plannerPaused = false;
  if (restore) { metadata.forgetDocumentMappings(); forgetPlannerMappings(); }
  connected.resetConnectedApps(runtime, { restore });
  runtime.cloudState(false, "Local account");
  await state();
}

const storageOptions = { storeFile: files.storeAttachment, deleteFile: files.purgeAttachment };
const loadFile = key => materializeCloudAttachment(key, auth.accountKey());

async function perform(action) {
  const preferences = JSON.parse(runtime.exportData()).settings || {};
  const preferenceKey = JSON.stringify([preferences.planner_enabled, preferences.outlook_categories_enabled, preferences.presentation_sync_enabled]);
  if (preferenceKey !== lastPreferences) { connected.connectedPreferencesChanged(); plannerPaused = false; lastPreferences = preferenceKey; }
  if (initial.demo && ["connectMicrosoft", "reconnectMicrosoft", "syncMicrosoft", "exportMicrosoft", "enableOutlookCategories", "enablePresentationSync"].includes(action.kind)) throw new Error("Demo mode does not connect or synchronize Microsoft accounts.");
  if (connected.connectedActions.includes(action.kind)) {
    await connected.connectedAppRequest(runtime, action.kind);
    if (!action.kind.startsWith("disable")) await connected.synchronizeConnectedApps(runtime, auth.accountKey(), JSON.parse(runtime.exportData()).settings, { force: true });
    return {};
  }
  switch (action.kind) {
    case "initialize": {
      metadata.recoverCommit();
      await files.cleanTrash(JSON.parse(runtime.exportData()));
      await state(); return {};
    }
    case "connectMicrosoft":
    case "reconnectMicrosoft": {
      runtime.flushEdits();
      runtime.hostStatus("Complete Microsoft sign-in in your system browser.");
      if (action.kind === "connectMicrosoft") await auth.connect(action.client_id || ""); else await auth.reconnect();
      const account = auth.account();
      runtime.cloudState(true, account.name || account.username || "Microsoft account");
      connected.restoreConnectedApps(runtime, auth.accountKey(), preferences);
      await state(); return {};
    }
    case "disconnectMicrosoft": await disconnect(); return {};
    case "signOutMicrosoft":
      try { await auth.logout(); } finally { await disconnect(); }
      return {};
    case "syncMicrosoft": {
      if (!auth.account()) throw new Error("Connect your Microsoft account first.");
      if (!action.automatic) runtime.cloudState(true, "Syncing…");
      const changed = await synchronize(runtime, auth.accountKey(), action.today);
      if (preferences.planner_enabled && auth.isWorkAccount() && (!plannerPaused || !action.automatic)) {
        try { await synchronizePlanner(runtime, auth.accountKey(), action.today, { force: !action.automatic }); plannerPaused = false; }
        catch (error) { plannerPaused = requiresSyncReview(error); if (!plannerPaused || !action.automatic) throw error; runtime.hostError(`Planner sync needs attention: ${error.message}`); }
      }
      const apps = await connected.synchronizeConnectedApps(runtime, auth.accountKey(), preferences, { force: !action.automatic });
      if (changed || apps.changed || !action.automatic) runtime.cloudState(true, `${auth.account().username || "Microsoft account"} · ${apps.issues.length ? "Connected app needs attention" : "Synced"}`);
      return { changed };
    }
    case "openMicrosoftToDo": {
      const url = auth.serviceEnvironment().todoUrl;
      if (!url) throw new Error("Microsoft To Do is not available in this Microsoft cloud.");
      ui("openExternalUrl", { url }); return {};
    }
    case "attach": {
      const bytes = readBounded(action.path, 25 * 1024 * 1024);
      const file = new File([bytes], action.name, { type: action.mime_type || "application/octet-stream" });
      const key = await files.saveAttachment(file);
      try { ui("addAttachment", { task_id: action.task_id, attachment: { id: 0, name: file.name, mime_type: file.type, size: file.size, storage_key: key } }); }
      catch (error) { await files.purgeAttachment(key); throw error; }
      return {};
    }
    case "openAttachment": {
      const file = await loadFile(action.key);
      if (!file) throw new Error("The attachment is unavailable. Connect its Microsoft account or attach the original file again.");
      atomicWrite(action.path, Buffer.from(await file.arrayBuffer())); return { saved: true };
    }
    case "deleteAttachment": await files.deleteAttachment(action.key); return {};
    case "exportArchive": {
      const blob = await exportPortable(runtime, { resolveFile: reference => loadFile(reference.storage_key) });
      atomicWrite(action.path, Buffer.from(await blob.arrayBuffer())); runtime.hostStatus("Backup with files exported."); return {};
    }
    case "exportMicrosoft": {
      const blob = await exportMicrosoftArchive(auth.accountKey());
      atomicWrite(action.path, Buffer.from(await blob.arrayBuffer())); runtime.hostStatus("Microsoft To Do archive exported."); return {};
    }
    case "exportPst": {
      const { exportPst } = await import("../web/backup/pst-export.js");
      const result = await exportPst(runtime, { resolveFile: reference => loadFile(reference.storage_key) });
      atomicWrite(action.path, Buffer.from(await result.blob.arrayBuffer()));
      runtime.hostStatus(`Outlook tasks exported.${result.warnings?.length ? ` ${result.warnings.join(" ")}` : ""}`); return {};
    }
    case "import":
    case "importMicrosoft":
    case "importPstReport":
    case "importPortableJson": {
      const expected = action.expected || runtime.exportData();
      if (runtime.exportData() !== expected) throw new Error("Tasks changed while the import was prepared. Choose the archive again.");
      beginImport(expected);
      try {
        let report;
        if (action.kind === "importPstReport") {
          const { normalizePstImport } = await import("../web/backup/pst.js");
          report = normalizePstImport(action.report);
        }
        const text = report ? JSON.stringify(report.archive) : action.json || readBounded(action.path, 256 * 1024 * 1024).toString("utf8");
        const result = action.kind === "importMicrosoft" ? await restoreMicrosoftArchive(runtime, text, storageOptions) : await restorePortable(runtime, text, storageOptions);
        await disconnect(true);
        runtime.hostStatus(report ? `Imported ${result.tasks} Outlook tasks as local copies. Microsoft sync is paused.${report.skipped_messages ? ` Skipped ${report.skipped_messages} non-task items.` : ""}${report.warnings.length ? ` ${report.warnings.join(" ")}` : ""}` : "Restored as a local copy. Microsoft sync is paused.");
        return result;
      } finally { endImport(); }
    }
    default: throw new Error(`Native service action is not supported: ${action.kind}`);
  }
}

metadata.recoverCommit();
send({ type: "ready" });
while (true) {
  const request = receive();
  if (!request || request.type === "shutdown") break;
  if (request.type !== "command") throw new Error("Invalid native command.");
  try {
    const value = await perform(request.action);
    send({ type: "result", id: request.id, value, connected: !!auth.account(), needsInteraction: auth.sessionStatus().needsInteraction });
  } catch (error) {
    send({ type: "result", id: request.id, error: error?.message || "The native operation failed.", review: requiresSyncReview(error), connected: !!auth.account(), needsInteraction: auth.sessionStatus().needsInteraction });
  }
}
await auth.disconnect();
