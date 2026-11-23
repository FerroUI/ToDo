import { mkdir, open, readFile, rename, rm, stat } from "node:fs/promises";
import { dirname } from "node:path";
import { hostname } from "node:os";
import { randomBytes } from "node:crypto";
import { HttpError } from "./security.mjs";
import { validateStore } from "./store-validation.mjs";

const MAX_STORE_BYTES = 64 * 1024 * 1024;

/** One writer, atomic rename, fsync before acknowledgement. No capabilities are
 * stored here: only their SHA-256 hashes. Fail closed on corrupt state. */
export class DurableStore {
  constructor(path, state, lock, io) { this.path = path; this.state = state; this.lock = lock; this.io = io; this.pending = Promise.resolve(); this.poisoned = false; }
  static async open(path, { io = { open, rename, rm } } = {}) {
    await mkdir(dirname(path), { recursive: true, mode: 0o700 });
    let lock;
    try { lock = await open(`${path}.lock`, "wx", 0o600); }
    catch (error) {
      if (error.code === "EEXIST") throw new Error("Reminder store is locked. Run one service instance; after a crash, verify the old process has stopped before removing its .lock file.");
      throw error;
    }
    try {
      await lock.writeFile(JSON.stringify({ pid: process.pid, host: hostname(), startedAt: Date.now() }));
      await lock.sync();
      let state = { version: 1, installations: {} };
      try {
        if ((await stat(path)).size > MAX_STORE_BYTES) throw new Error("Reminder store is too large.");
        state = JSON.parse(await readFile(path, "utf8"));
      } catch (error) { if (error.code !== "ENOENT") throw error; }
      validateStore(state);
      return new DurableStore(path, state, lock, io);
    } catch (error) { await lock.close(); await rm(`${path}.lock`, { force: true }); throw error; }
  }
  snapshot() {
    if (this.poisoned) throw new Error("Reminder storage durability is uncertain; restart after checking the filesystem.");
    return structuredClone(this.state);
  }
  transact(change) {
    const operation = this.pending.then(async () => {
      if (this.poisoned) throw new Error("Reminder storage durability is uncertain; restart after checking the filesystem.");
      const next = structuredClone(this.state);
      const result = change(next);
      if (result instanceof Promise) throw new Error("Store transactions must not await external work.");
      const bytes = JSON.stringify(next);
      if (Buffer.byteLength(bytes) > MAX_STORE_BYTES) throw new HttpError(503, "Reminder storage quota reached.");
      const temporary = `${this.path}.${randomBytes(12).toString("hex")}.tmp`;
      let renamed = false;
      try {
        const file = await this.io.open(temporary, "wx", 0o600);
        try { await file.writeFile(bytes); await file.sync(); } finally { await file.close(); }
        await this.io.rename(temporary, this.path); renamed = true;
        const directory = await this.io.open(dirname(this.path), "r");
        try { await directory.sync(); } finally { await directory.close(); }
        this.state = next;
        return result;
      } catch (error) {
        // The rename may already be durable even when directory fsync fails.
        // Never let a queued mutation overwrite it from the previous snapshot.
        if (renamed) this.poisoned = true;
        throw error;
      } finally { await this.io.rm(temporary, { force: true }).catch(() => {}); }
    });
    this.pending = operation.catch(() => {});
    return operation;
  }
  async close() {
    await this.pending;
    if (this.lock) { await this.lock.close(); this.lock = null; await rm(`${this.path}.lock`, { force: true }); }
  }
}

export class MemoryStore {
  constructor() { this.state = { version: 1, installations: {} }; this.pending = Promise.resolve(); }
  snapshot() { return structuredClone(this.state); }
  transact(change) {
    const operation = this.pending.then(() => { const next = structuredClone(this.state); const result = change(next); this.state = next; return result; });
    this.pending = operation.catch(() => {}); return operation;
  }
}
