// Installed before the application loads. Observers remain passive and never
// capture, cancel, synthesize, or delay an input event.
export function browserInputTrace() {
  const events = [], limit = 512;
  let total = 0;
  const element = value => value?.tagName
    ? `${value.tagName.toLowerCase()}${value.id ? `#${value.id}` : ""}` : value === globalThis ? "window" : "document";
  for (const type of ["pointerdown", "pointermove", "pointerup", "pointercancel",
    "gotpointercapture", "lostpointercapture", "mousedown", "mousemove", "mouseup",
    "focus", "blur", "visibilitychange", "popstate", "wheel",
    "dragenter", "dragover", "dragleave", "drop", "dragend"]) {
    globalThis.addEventListener(type, event => {
      const entry = { sequence: ++total, time: performance.now(), type,
        target: element(event.target), trusted: event.isTrusted,
        focus: document.hasFocus(), visibility: document.visibilityState,
        activeElement: element(document.activeElement) };
      for (const key of ["pointerId", "pointerType", "isPrimary", "button", "buttons",
        "clientX", "clientY", "pressure", "deltaX", "deltaY", "ctrlKey", "shiftKey", "altKey", "metaKey"]) {
        if (event[key] !== undefined) entry[key] = event[key];
      }
      if (event.pointerId !== undefined && event.target?.hasPointerCapture) {
        entry.captured = event.target.hasPointerCapture(event.pointerId);
      }
      events.push(entry);
      if (events.length > limit) events.shift();
    }, { capture: true, passive: true });
  }
  globalThis.__todoInputTrace = { read: () => ({ events: events.slice(), total,
    dropped: Math.max(0, total - limit), focus: document.hasFocus(),
    visibility: document.visibilityState, activeElement: element(document.activeElement) }) };
}

export const inputTraceScript = `(${browserInputTrace.toString()})()`;

// Record the failing state before releasing the pressed mouse. Cleanup covers
// failures while starting the drag as well as failures after it has started.
export async function runPointerStage(ctx, stage, operation) {
  const state = { stage, phase: "begin" };
  let primary;
  console.log(`gesture: ${stage}`);
  try {
    return await operation(state);
  } catch (error) {
    primary = error;
    const observations = await Promise.allSettled([
      Promise.resolve().then(() => ctx.inputTrace()),
      Promise.resolve().then(() => ctx.snapshot()),
    ]);
    error.inputDiagnostics = { ...state };
    observations.forEach((result, index) => {
      const name = ["trace", "snapshot"][index];
      error.inputDiagnostics[result.status === "fulfilled" ? name : `${name}Error`]
        = result.status === "fulfilled" ? result.value : String(result.reason?.stack || result.reason);
    });
    throw error;
  } finally {
    if (state.mouse) {
      try { await ctx.page.mouseUp(state.mouse.x, state.mouse.y); }
      catch (error) {
        if (!primary) throw error;
        primary.inputDiagnostics.cleanupError = String(error?.stack || error);
      }
    }
  }
}
