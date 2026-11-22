const PREVIEW_HANDOFF = "ferroui.todo.preview-handoff:";

function understandsPreview(worker, timeout) {
  return new Promise(resolve => {
    const channel = new MessageChannel();
    const finish = value => { clearTimeout(timer); channel.port1.close(); channel.port2.close(); resolve(value); };
    const timer = setTimeout(() => finish(false), timeout);
    channel.port1.onmessage = event => finish(event.data?.type === "TODO_PREVIEW_READY");
    try { worker.postMessage({ type: "TODO_PREVIEW_CHECK" }, [channel.port2]); }
    catch { finish(false); }
  });
}

export async function prepareOfflinePreview({ href, serviceWorker, storage, reload }, timeout = 400) {
  const url = new URL(href);
  const scope = new URL("./", url);
  const marker = PREVIEW_HANDOFF + scope.href;
  const forget = () => { try { storage().removeItem(marker); } catch {} };
  const worker = serviceWorker?.controller;
  if (!url.searchParams.has("nooffline") || !worker) { forget(); return true; }
  if (await understandsPreview(worker, timeout)) { forget(); return true; }

  const registration = await serviceWorker.getRegistration(scope.href);
  if (!serviceWorker.controller) { forget(); return true; }
  if (serviceWorker.controller !== worker) throw new Error("The offline installation changed. Reload this preview.");
  const expected = new URL("app-sw.js", scope);
  const controller = new URL(worker.scriptURL);
  // Never remove another application or a broader origin-level registration.
  if (registration?.scope !== scope.href || controller.origin !== expected.origin || controller.pathname !== expected.pathname) {
    throw new Error("This preview is controlled by another app. Open it in a separate browser profile to preview without offline caching.");
  }
  let previous;
  try { previous = storage().getItem(marker); storage().setItem(marker, "1"); }
  catch { throw new Error("Allow session storage, then reload this preview to bypass the older offline installation."); }
  if (previous) throw new Error("The older offline installation still controls this preview. Close this app's tabs and reopen the preview.");
  if (!await registration.unregister()) { forget(); throw new Error("The offline installation could not be bypassed. Reload this preview."); }
  reload();
  return false;
}

export function registerOffline(serviceWorker, href) {
  if (!serviceWorker || new URL(href).searchParams.has("nooffline")) return Promise.resolve(null);
  const scope = new URL("./", href);
  return serviceWorker.register(new URL("app-sw.js", scope).href, { scope: scope.href, updateViaCache: "none" });
}
