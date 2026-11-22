export function requiresSyncReview(error) {
  return !!error.requiresInteraction || [400, 401, 403, 404, 409, 412].includes(error.status)
    || /(?:sync|checklist) conflict|linked to another Microsoft account|another To Do tab|changed here and|deleted here and|interrupted.*check Microsoft|keep a local copy|history.*unreadable|did not return a revision|returned an invalid/i.test(error.message || "");
}

// One foreground timer; local saves are debounced and hidden/offline tabs do no work.
export function automaticSync({ run, ready, report, timer = setTimeout, cancel = clearTimeout,
  interval = 5_000, debounce = 900 }) {
  let scheduled, running = false, pending = false, stopped = false, blocked = false, paused = false;
  let failures = 0, lastError = "";
  const clear = () => { if (scheduled !== undefined) cancel(scheduled); scheduled = undefined; };
  const schedule = delay => {
    clear();
    if (!stopped && !blocked && !paused && ready()) scheduled = timer(() => { scheduled = undefined; void tick(); }, delay);
  };
  const tick = async () => {
    if (stopped || blocked || paused || !ready()) return;
    if (running) { pending = true; return; }
    running = true;
    try {
      await run(); failures = 0; lastError = "";
    } catch (error) {
      failures++;
      // Authentication and service conflicts need a user change or explicit retry.
      blocked = requiresSyncReview(error);
      if (lastError !== error.message) { lastError = error.message; report(error); }
    } finally {
      running = false;
      const delay = pending ? debounce : Math.min(60_000, interval * 2 ** Math.min(failures, 4));
      pending = false;
      schedule(delay);
    }
  };
  return {
    start() { stopped = false; paused = false; schedule(0); },
    changed() { blocked = false; failures = 0; if (running) pending = true; else schedule(debounce); },
    resume() { paused = false; if (!running) schedule(0); },
    retry() { blocked = false; failures = 0; lastError = ""; if (!running) schedule(0); },
    pause() { paused = true; clear(); },
    stop() { stopped = true; pending = false; clear(); },
  };
}
