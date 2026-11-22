import { randomBytes } from "node:crypto";
import { accountIdentity, cacheScope, decryptCache, encryptCache, selectedAccount } from "./cache-format.mjs";
import { cacheFiles } from "./cache-storage.mjs";
import { credential } from "./credentials.mjs";

export class SecureTokenCache {
  constructor(directory, configuration, { vault = credential, files } = {}) {
    this.scope = cacheScope(directory, configuration);
    this.files = files || cacheFiles(directory, this.scope);
    this.vault = vault;
    this.epoch = 0; this.contexts = new WeakMap(); this.pending = Promise.resolve();
    this.active = false; this.paused = 0; this.failed = false; this.mode = "ready";
    this.plugin = {
      beforeCacheAccess: async context => {
        const epoch = this.epoch;
        const saved = await this.load();
        if (epoch !== this.epoch) return;
        this.contexts.set(context, epoch);
        if (saved && !this.deserialized) {
          this.deserialized = true;
          try { context.tokenCache.deserialize(saved.cache); }
          catch { this.unavailable(); this.account = undefined; this.active = false; }
        }
      },
      afterCacheAccess: async context => {
        if (context.cacheHasChanged && this.contexts.get(context) === this.epoch && this.active && !this.paused && !this.failed) {
          await this.save(context.tokenCache.serialize());
        }
      }
    };
  }

  unavailable() { this.failed = true; this.mode = "session"; }

  async key(create = false) {
    if (this.keyBytes) return this.keyBytes;
    if (this.failed) throw new Error("Secure sign-in storage is unavailable.");
    if (this.keyRequest) return this.keyRequest;
    const epoch = this.epoch;
    this.keyRequest = (async () => {
      let value = await this.vault("read", this.scope.id);
      if (epoch !== this.epoch) throw new Error("Secure sign-in storage was cancelled.");
      if (value === null && create) {
        const generated = randomBytes(32);
        try { value = generated.toString("hex"); await this.vault("write", this.scope.id, value); }
        finally { generated.fill(0); }
      }
      if (epoch !== this.epoch) throw new Error("Secure sign-in storage was cancelled.");
      if (typeof value !== "string" || !/^[a-f0-9]{64}$/.test(value)) throw new Error("Secure sign-in storage is unavailable.");
      this.keyBytes = Buffer.from(value, "hex");
      return this.keyBytes;
    })();
    try { return await this.keyRequest; }
    finally { this.keyRequest = undefined; }
  }

  load() {
    if (this.loading) return this.loading;
    const epoch = this.epoch;
    this.loading = (async () => {
      try {
        const marker = this.files.readMarker();
        if (!marker.enabled) return null;
        const key = await this.key();
        if (epoch !== this.epoch) return null;
        const payload = decryptCache(this.scope, marker.generation, this.files.readPayload(), key);
        this.generation = marker.generation; this.account = payload.account;
        this.active = true; this.mode = "protected";
        return payload;
      } catch { if (epoch === this.epoch) this.unavailable(); return null; }
    })();
    return this.loading;
  }

  async restore(tokenCache) {
    const epoch = this.epoch;
    await this.load();
    if (epoch !== this.epoch || !this.active || !this.account || this.failed) return null;
    const accounts = await tokenCache.getAllAccounts();
    if (epoch !== this.epoch || !this.active || !this.account || this.failed) return null;
    const account = selectedAccount(accounts, this.account);
    if (!account) {
      this.revoke();
      this.mode = "reconnect";
      return null;
    }
    return account;
  }

  pauseWrites() {
    this.paused++;
    return () => { this.paused = Math.max(0, this.paused - 1); };
  }

  // Synchronous by design: restorePortable's final document commit is also
  // synchronous. A durable tombstone must precede that commit or any cleanup.
  revoke() {
    this.epoch++; this.active = false; this.account = undefined;
    this.loading = Promise.resolve(null);
    this.files.writeMarker({ enabled: false });
    this.mode = "disabled";
  }

  retry() {
    if (!this.failed && this.mode !== "reconnect") return;
    this.revoke();
    this.keyBytes?.fill(0); this.keyBytes = undefined;
    this.failed = false; this.deserialized = true; this.mode = "ready";
  }

  async select(account, cache) {
    const identity = accountIdentity(account);
    this.revoke(); // A failed replacement must never resume an older account.
    const epoch = this.epoch, generation = randomBytes(16).toString("hex");
    try {
      if (this.failed) { this.mode = "session"; return false; }
      const key = await this.key(true);
      if (epoch !== this.epoch) return false;
      const payload = encryptCache(this.scope, generation, identity, cache, key);
      this.files.writePayload(payload);
      this.files.writeMarker({ enabled: true, generation });
      this.generation = generation; this.account = identity; this.active = true; this.mode = "protected";
      return true;
    } catch {
      if (epoch === this.epoch) {
        // A rename may have succeeded before directory durability failed.
        // Never claim session-only until a disabled marker is durable.
        try { this.revoke(); this.unavailable(); }
        catch { this.failed = true; this.mode = "storage-error"; }
      }
      return false;
    }
  }

  save(cache) {
    const epoch = this.epoch;
    this.pending = this.pending.then(async () => {
      if (epoch !== this.epoch || !this.active || this.paused || this.failed) return;
      try {
        const key = await this.key();
        if (epoch !== this.epoch || !this.active) return;
        this.files.writePayload(encryptCache(this.scope, this.generation, this.account, cache, key));
      } catch {
        if (epoch === this.epoch) {
          try { this.revoke(); this.unavailable(); }
          catch { this.failed = true; this.mode = "storage-error"; }
        }
      }
    });
    return this.pending;
  }

  async forget() {
    this.revoke();
    await Promise.allSettled([this.pending, this.keyRequest]);
    let complete = true;
    try { this.files.deletePayload(); } catch { complete = false; }
    try { await this.vault("delete", this.scope.id); } catch { complete = false; }
    this.dispose();
    this.mode = complete ? "disabled" : "cleanup";
    return complete;
  }

  dispose() {
    this.epoch++; this.active = false; this.account = undefined;
    this.keyBytes?.fill(0); this.keyBytes = undefined;
  }

  hint() {
    if (this.mode === "protected") return "Your Microsoft sign-in is saved securely on this device using the operating system credential store.";
    if (this.mode === "session") return "Secure storage is unavailable or saved sign-in could not be verified. This sign-in lasts only until To Do exits; reconnect to retry secure storage.";
    if (this.mode === "reconnect") return "The saved Microsoft account could not be identified unambiguously. Reconnect Microsoft to choose the account again.";
    if (this.mode === "cleanup") return "Automatic sign-in is disabled. Some protected cache cleanup could not complete because secure storage is unavailable.";
    if (this.mode === "storage-error") return "Saved sign-in could not be updated or disabled. Check access to your To Do data directory before signing out.";
    return "Microsoft sign-in opens your system browser. It is remembered on this device when the operating system credential store is available.";
  }
}
