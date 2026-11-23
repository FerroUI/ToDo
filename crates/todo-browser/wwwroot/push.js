import "./push-storage.js";
import { PushClient } from "./push-client.js";
import { workspaceAccount } from "./cloud/workspaces.js";
import { notify } from "./notifications.js";
import { workerSupportsPush } from "./push-protocol.js";

let client;
export function attachPush(runtime) {
  const url = new URL(location.href), scope = new URL(".", url).href;
  const demo = url.searchParams.get("demo") === "1", preview = url.searchParams.has("nooffline");
  const capable = !!(globalThis.isSecureContext && navigator.serviceWorker && globalThis.PushManager && globalThis.Notification && globalThis.indexedDB);
  const supported = capable && !demo && !preview;
  const reason = demo ? "Background push is disabled in the demo. Configure it in your personal app workspace."
    : preview ? "Background push is unavailable in an offline-disabled preview. Open the installed app to configure it."
      : "This browser does not support background push. On iPhone, install To Do on your Home Screen first.";
  client = new PushClient({ scope, store: new globalThis.TodoPushStore(scope), supported, reason,
    readWorkspace() {
      return { account: demo ? "demo" : workspaceAccount(), json: localStorage.getItem(demo ? "ferroui.todo.demo.v1" : "ferroui.todo.v1") || '{"tasks":[]}',
        blocked: !demo && localStorage.getItem("ferroui.todo.workspaces.v1.switch") !== null };
    },
    async registration() {
      const registration = await navigator.serviceWorker?.getRegistration(scope);
      if (registration?.scope !== scope || !registration.active || !registration.pushManager) throw new Error("The app's offline installation is not ready. Reload once, then enable background reminders.");
      if (!await workerSupportsPush(registration.active)) throw new Error("Close older To Do tabs and reopen the app to finish installing background reminder support.");
      return registration;
    },
    permission: () => globalThis.Notification?.permission,
    requestPermission: () => Notification.requestPermission(),
    lock: (name, action) => navigator.locks ? navigator.locks.request(name, action) : action(),
    onState: state => runtime.pushState?.(JSON.stringify(state)),
  });
  void client.start().catch(error => runtime.hostError(error?.message || String(error)));
  const changed = () => client.changed();
  window.addEventListener("storage", event => {
    if (event.key === null || event.key === "ferroui.todo.v1" || event.key?.startsWith("ferroui.todo.workspaces.v1.")) changed();
  });
  window.addEventListener("online", changed);
  window.addEventListener("pageshow", changed);
  document.addEventListener("visibilitychange", () => { if (document.visibilityState === "visible") changed(); });
  if (supported) {
    navigator.permissions?.query({ name: "notifications" }).then(permission => { permission.onchange = changed; }).catch(() => {});
    // Refresh the local timezone and extend a near-expiring installation while
    // the app remains open. Delivery while closed is owned by the service.
    setInterval(changed, 60_000);
  }
}
export function pushDocumentChanged() { client?.changed(); }
export function pushDocumentImported() { client?.imported(); }
export async function pushRequest(action) {
  if (!client) throw new Error("Background reminders are starting. Try again shortly.");
  if (action.kind === "enablePush") return client.enable(action);
  if (action.kind === "disablePush") return client.disable();
  if (action.kind === "refreshPush") return client.refresh();
  throw new Error("Unknown background reminder action.");
}
export async function notifyWithPush(action) {
  const handled = client && await client.foreground(action, delivery => notify(action.title, action.body, delivery));
  if (!handled) await notify(action.title, action.body);
}
