// Compare with the last successfully read/written document before each synchronous save.
// Never advance the baseline from a storage event: doing that would authorize stale edits.
export function createDocumentStorage(getStorage, key) {
  let observed;
  let loaded = false;

  return {
    load() {
      observed = getStorage().getItem(key);
      loaded = true;
      return observed ?? "";
    },
    save(contents) {
      if (!loaded) {
        throw new Error("Task storage was not read successfully. Export your tasks to keep a backup, then reload this tab before saving.");
      }
      const storage = getStorage();
      if (storage.getItem(key) !== observed) {
        throw new Error("Tasks changed in another tab. Your edits are still open here. Export a backup of this tab, then reload it to read the newer saved tasks before making more changes.");
      }
      storage.setItem(key, contents);
      // A failed write must leave the baseline unchanged so a later retry is safe.
      observed = contents;
    },
  };
}
