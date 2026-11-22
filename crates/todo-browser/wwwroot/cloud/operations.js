// One host queue owns cloud/account transitions. A refresh joins the pending sync;
// an account change is a barrier, so later syncs run against the resulting account.
export function serializeMicrosoft(run) {
  let tail = Promise.resolve();
  let syncing;
  return (action, today) => {
    if (action.kind === "syncMicrosoft" && syncing) {
      if (!action.automatic && syncing.action.automatic) {
        if (!syncing.started) {
          syncing.action.automatic = false;
          return syncing.operation;
        }
        // A manual refresh must reach forceful provider refresh and surface its
        // errors even when a quiet automatic pass has already started.
      } else return syncing.operation;
    }
    const record = { action: { ...action }, started: false };
    const operation = tail.then(() => { record.started = true; return run(record.action, today); });
    record.operation = operation;
    tail = operation.catch(() => {});
    syncing = action.kind === "syncMicrosoft" ? record : undefined;
    const clear = () => { if (syncing === record) syncing = undefined; };
    void operation.then(clear, clear);
    return operation;
  };
}
