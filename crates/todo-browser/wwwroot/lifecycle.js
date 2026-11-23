let current;

// Browser window blur intentionally keeps FerroUI's focused editor. Interrupt
// gestures separately; hiding/leaving the document still commits visible drafts.
export function createLifecycle({ host, cancel, recover, cancelPen = () => {}, flush, tick, reportError }) {
  const listeners = [];
  const invoke = action => {
    try { action(); } catch (error) { reportError(error); }
  };
  const listen = (target, type, callback, capture = false) => {
    target.addEventListener(type, callback, capture);
    listeners.push(() => target.removeEventListener(type, callback, capture));
  };
  const interrupt = () => { invoke(cancel); invoke(flush); };
  listen(host, "blur", () => invoke(cancel));
  listen(host, "focus", () => invoke(tick));
  listen(host, "pagehide", interrupt);
  listen(host, "beforeunload", interrupt);
  listen(host.document, "visibilitychange", () => {
    if (host.document.hidden) interrupt();
    else invoke(tick);
  });
  const surface = host.document.getElementById("out");
  if (surface) listen(surface, "pointerdown", event => {
    // A fresh primary Down starts a new touch sequence (Pointer Events §4.1.2).
    // Focus, surviving contacts and synthetic events cannot clear quarantine.
    // Capture runs before FerroUI's listener so old native pointers retire first.
    if (event.isTrusted && event.pointerType === "touch" && event.isPrimary) invoke(recover);
  }, true);
  if (surface) listen(surface, "pointercancel", event => {
    // The pinned native browser bridge retires touch cancellation itself. Pen
    // cancellation uses the recorded native PenDevice without fabricating Up.
    if (event.isTrusted && event.pointerType === "pen" && Number.isSafeInteger(event.pointerId))
      invoke(() => cancelPen(event.pointerId));
  }, true);
  const timer = host.setInterval(() => invoke(tick), 30_000);
  invoke(tick);
  return () => {
    for (const remove of listeners.splice(0)) remove();
    host.clearInterval(timer);
  };
}

export function attachLifecycle(runtime, tick, reportError, host = window) {
  current?.();
  current = createLifecycle({ host, tick, reportError,
    cancel: () => runtime.cancelGestures(), recover: () => runtime.recoverTouchSequence(),
    cancelPen: id => runtime.cancelPenPointer(id),
    flush: () => runtime.flushEdits() });
  return current;
}
