const DATABASE = "ferroui-todo-files";
const STORE = "attachments";
const TRASH = "trash";
let connection;

async function database() {
  if (!connection) connection = new Promise((resolve, reject) => {
    const open = indexedDB.open(DATABASE, 2);
    open.onupgradeneeded = () => {
      for (const name of [STORE, TRASH]) if (!open.result.objectStoreNames.contains(name)) open.result.createObjectStore(name);
    };
    open.onsuccess = () => { resolve(open.result); cleanTrash(open.result); };
    open.onerror = () => { connection = undefined; reject(open.error); };
    open.onblocked = () => { connection = undefined; reject(new Error("Close another To Do tab to open file storage.")); };
  });
  return connection;
}

async function transact(mode, action) {
  const db = await database();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE, mode);
    const request = action(tx.objectStore(STORE));
    let result;
    request.onsuccess = () => { result = request.result; };
    tx.oncomplete = () => resolve(result);
    tx.onerror = () => reject(tx.error || request.error);
    tx.onabort = () => reject(tx.error || new Error("File storage was interrupted."));
  });
}

export async function saveAttachment(file) {
  if (file.size > 25 * 1024 * 1024) throw new Error("Attachments must be 25 MB or smaller.");
  const key = crypto.randomUUID();
  await transact("readwrite", store => store.put(file, key));
  return key;
}

export async function deleteAttachment(key) {
  const db = await database();
  return new Promise((resolve, reject) => {
    const tx = db.transaction([STORE, TRASH], "readwrite");
    const request = tx.objectStore(STORE).get(key);
    request.onsuccess = () => {
      if (request.result) tx.objectStore(TRASH).put({ file: request.result, deletedAt: Date.now() }, key);
      tx.objectStore(STORE).delete(key);
    };
    tx.oncomplete = resolve;
    tx.onerror = () => reject(tx.error);
  });
}

export async function openAttachment(key) {
  let file = await transact("readonly", store => store.get(key));
  if (!file) {
    const db = await database();
    file = await new Promise((resolve, reject) => {
      const tx = db.transaction([STORE, TRASH], "readwrite");
      const request = tx.objectStore(TRASH).get(key);
      let recovered;
      request.onsuccess = () => {
        recovered = request.result?.file;
        if (recovered) { tx.objectStore(STORE).put(recovered, key); tx.objectStore(TRASH).delete(key); }
      };
      tx.oncomplete = () => resolve(recovered);
      tx.onerror = () => reject(tx.error);
    });
  }
  if (!file) throw new Error("This file is not stored on this device. Attach it again from the original file.");
  downloadBlob(file, file.name || "attachment");
}

function cleanTrash(db) {
  let retained;
  try {
    const documents = ["ferroui.todo.v1", "ferroui.todo.demo.v1"].map(key => JSON.parse(localStorage.getItem(key) || "{}"));
    retained = new Set(documents.flatMap(doc => (doc.tasks || []).flatMap(task => (task.attachments || []).map(file => file.storage_key))));
  } catch { return; }
  const tx = db.transaction([STORE, TRASH], "readwrite");
  const request = tx.objectStore(TRASH).openCursor();
  request.onsuccess = () => {
    const cursor = request.result;
    if (!cursor) return;
    if (retained.has(cursor.key)) { tx.objectStore(STORE).put(cursor.value.file, cursor.key); cursor.delete(); }
    else if (Date.now() - cursor.value.deletedAt > 30 * 24 * 60 * 60 * 1000) cursor.delete();
    cursor.continue();
  };
}

export function downloadBlob(blob, filename) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 30_000);
}

export function chooseFile(accept, multiple = false) {
  return new Promise(resolve => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = accept;
    input.multiple = multiple;
    input.style.display = "none";
    document.body.append(input);
    input.addEventListener("change", () => { const files = [...input.files]; input.remove(); resolve(files); }, { once: true });
    input.addEventListener("cancel", () => { input.remove(); resolve([]); }, { once: true });
    input.click();
  });
}
