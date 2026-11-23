// A page can be running a newer immutable deployment while an older worker is
// still active. Enrollment must not target a worker that cannot consume pushes.
export function workerSupportsPush(worker, timeout = 1000) {
  return new Promise(resolve => {
    const channel = new MessageChannel();
    let settled = false;
    const finish = value => {
      if (settled) return; settled = true;
      clearTimeout(timer); channel.port1.close(); channel.port2.close(); resolve(value);
    };
    const timer = setTimeout(() => finish(false), timeout);
    channel.port1.onmessage = event => finish(event.data?.type === "TODO_PUSH_READY" && event.data.version === 1);
    try { worker.postMessage({ type: "TODO_PUSH_CHECK", version: 1 }, [channel.port2]); }
    catch { finish(false); }
  });
}
