// One host queue owns cloud/account transitions. A refresh joins the pending sync;
// an account change is a barrier, so later syncs run against the resulting account.
export function serializeMicrosoft(run) {
  let tail = Promise.resolve();
  let syncing;
  return (action, today) => {
    if (action.kind === "syncMicrosoft" && syncing) return syncing;
    const operation = tail.then(() => run(action, today));
    tail = operation.catch(() => {});
    syncing = action.kind === "syncMicrosoft" ? operation : undefined;
    const clear = () => { if (syncing === operation) syncing = undefined; };
    void operation.then(clear, clear);
    return operation;
  };
}
