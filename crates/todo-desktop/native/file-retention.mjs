const PREFIX = "ferroui.todo.graph.v1.";

// A cloud move can stage files before the document accepts their new keys.
// Preserve both explicit mappings and source generations needed to resume an
// interrupted move, including metadata belonging to a disconnected account.
export function synchronizationFileKeys(storage, cloudAttachmentKey) {
  const retained = new Set();
  for (let index = 0; index < storage.length; index++) {
    const key = storage.key(index);
    if (!key?.startsWith(PREFIX)) continue;
    const account = key.slice(PREFIX.length);
    if (["device", "document-account"].includes(account)) continue;
    if (account === "commit") throw new Error("Recover the pending Microsoft synchronization commit before cleaning attachment storage.");
    let metadata;
    try { metadata = JSON.parse(storage.getItem(key)); }
    catch { throw new Error("Microsoft synchronization metadata is unreadable. Attachment cleanup was paused."); }
    if (!metadata || metadata.version !== 1 || !metadata.tasks || !metadata.lists) throw new Error("Microsoft synchronization metadata is unreadable. Attachment cleanup was paused.");
    const pending = [metadata];
    while (pending.length) {
      const value = pending.pop();
      for (const [field, entry] of Object.entries(value)) {
        if ((field === "storage_key" || field === "storageKey") && typeof entry === "string") retained.add(entry);
        else if (entry && typeof entry === "object") pending.push(entry);
      }
    }
    const moves = [...(metadata.pendingDeletes || []), ...Object.values(metadata.pendingTasks || {}).map(value => value.move).filter(Boolean)];
    for (const move of moves) {
      if (!move.sourceListId || !move.sourceTaskId) continue;
      for (const file of move.source?.attachments || []) retained.add(cloudAttachmentKey(account, move.sourceListId, move.sourceTaskId, file));
    }
  }
  return retained;
}
