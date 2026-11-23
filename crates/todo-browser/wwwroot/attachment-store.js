const DATABASE = "ferroui-todo-files";
const STORE = "attachments";
const TRASH = "trash";

function savedAttachmentKeys() {
  const documents = ["ferroui.todo.v1", "ferroui.todo.demo.v1"].map(key => JSON.parse(localStorage.getItem(key) || "{}"));
  return new Set(documents.flatMap(doc => (doc.tasks || []).flatMap(task => (task.attachments || []).map(file => file.storage_key))));
}

// Keep the browser connection and its transactions behind one lifecycle boundary.
// A blocked/closed connection is replaceable; a transaction is successful only
// after commit, including reads that recover a file retained for undo.
export function createAttachmentStore({
  openDatabase = () => indexedDB.open(DATABASE, 2),
  retainedKeys = savedAttachmentKeys,
  now = Date.now,
} = {}) {
  let connection;

  function database() {
    if (connection) return connection.promise;
    const attempt = {};
    connection = attempt;
    attempt.promise = new Promise((resolve, reject) => {
      let request, failed = false;
      const forget = () => { if (connection === attempt) connection = undefined; };
      const fail = error => { failed = true; forget(); reject(error); };
      try { request = openDatabase(); }
      catch (error) { fail(error); return; }
      request.onupgradeneeded = () => {
        // A blocked request cannot be cancelled. If it eventually resumes after
        // we have rejected it, abort its upgrade instead of leaving a hidden tab
        // connection holding the next attempt open.
        if (failed) { request.transaction.abort(); return; }
        for (const name of [STORE, TRASH]) {
          if (!request.result.objectStoreNames.contains(name)) request.result.createObjectStore(name);
        }
      };
      request.onsuccess = () => {
        const db = request.result;
        if (failed) { db.close(); return; }
        db.onversionchange = () => { forget(); db.close(); };
        db.onclose = forget;
        resolve(db);
        // Cleanup is opportunistic. Storage denial or an already closing
        // connection must never prevent the foreground operation from settling.
        try { cleanTrash(db); } catch {}
      };
      request.onerror = () => fail(request.error || new Error("File storage could not be opened."));
      request.onblocked = () => fail(new Error("Close another To Do tab to open file storage, then try again."));
    });
    return attempt.promise;
  }

  async function transaction(stores, mode, action) {
    const db = await database();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(stores, mode);
      let result = () => undefined;
      tx.oncomplete = () => resolve(result());
      tx.onerror = event => reject(tx.error || event?.target?.error || new Error("File storage failed."));
      tx.onabort = () => reject(tx.error || new Error("File storage was interrupted. Try again."));
      try { result = action(tx); }
      catch (error) { try { tx.abort(); } catch {} reject(error); }
    });
  }

  function cleanTrash(db) {
    const retained = retainedKeys();
    const tx = db.transaction([STORE, TRASH], "readwrite");
    const request = tx.objectStore(TRASH).openCursor();
    request.onsuccess = () => {
      const cursor = request.result;
      if (!cursor) return;
      if (retained.has(cursor.key)) { tx.objectStore(STORE).put(cursor.value.file, cursor.key); cursor.delete(); }
      else if (now() - cursor.value.deletedAt > 30 * 24 * 60 * 60 * 1000) cursor.delete();
      cursor.continue();
    };
  }

  return {
    put(key, file) {
      return transaction(STORE, "readwrite", tx => {
        tx.objectStore(STORE).put(file, key);
        return () => undefined;
      });
    },
    remove(key) {
      return transaction([STORE, TRASH], "readwrite", tx => {
        const request = tx.objectStore(STORE).get(key);
        request.onsuccess = () => {
          if (request.result) tx.objectStore(TRASH).put({ file: request.result, deletedAt: now() }, key);
          tx.objectStore(STORE).delete(key);
        };
        return () => undefined;
      });
    },
    async read(key) {
      const file = await transaction(STORE, "readonly", tx => {
        const request = tx.objectStore(STORE).get(key);
        return () => request.result;
      });
      if (file) return file;
      return transaction([STORE, TRASH], "readwrite", tx => {
        const request = tx.objectStore(TRASH).get(key);
        request.onsuccess = () => {
          const recovered = request.result?.file;
          if (recovered) { tx.objectStore(STORE).put(recovered, key); tx.objectStore(TRASH).delete(key); }
        };
        return () => request.result?.file || null;
      });
    },
  };
}
