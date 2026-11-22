import * as auth from "./cloud/auth.js";
import { graph } from "./cloud/client.js";
import { recoverCommit } from "./cloud/metadata.js";
import { synchronize } from "./cloud/sync.js";
import { serializeMicrosoft } from "./cloud/operations.js";

let runtime;
let startupError;
let restored;
const operations = serializeMicrosoft(performRequest);

export async function prepareMicrosoft() {
  try { recoverCommit(); restored = await auth.restore(); }
  catch (error) { startupError = error; }
}

export async function attachMicrosoft(value) {
  runtime = value;
  if (startupError) runtime.hostError(startupError.message);
  if (!restored) return;
  await operations({ kind: "restoreMicrosoft" });
}

export function microsoftRequest(action, today) {
  return operations(action, today);
}

async function performRequest(action, today) {
  if (action.kind === "restoreMicrosoft") {
    try {
      const user = await graph("me?$select=id,displayName,userPrincipalName");
      runtime.cloudState(true, user.displayName || user.userPrincipalName || restored.username);
    } catch (error) { runtime.cloudState(false, "Sign-in required"); runtime.hostError(error.message); }
    return;
  }
  if (new URLSearchParams(location.search).get("demo") === "1") throw new Error("Open the normal app to connect an account; demo tasks are kept separate.");
  if (action.kind === "connectMicrosoft") return auth.connect(action.client_id);
  if (action.kind === "disconnectMicrosoft") {
    await auth.disconnect(); runtime.cloudState(false, "Local account"); return;
  }
  if (action.kind !== "syncMicrosoft") throw new Error("Unknown Microsoft account action.");
  const account = auth.account();
  if (!account) throw new Error("Connect your Microsoft account first.");
  const run = async lock => {
    if (!lock) throw new Error("Another tab is synchronizing. Try again when it finishes.");
    runtime.cloudState(true, "Syncing…");
    try {
      await synchronize(runtime, account.homeAccountId, today);
      runtime.cloudState(true, `${account.username || "Microsoft account"} · Synced`);
    } catch (error) {
      runtime.cloudState(true, `${account.username || "Microsoft account"} · Sync needs attention`);
      throw error;
    }
  };
  if (navigator.locks) await navigator.locks.request("ferroui-todo-microsoft-sync", { ifAvailable: true }, run);
  else throw new Error("This browser cannot safely coordinate cloud synchronization. Use a current browser with Web Locks support.");
}
