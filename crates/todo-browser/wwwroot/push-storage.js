// Shared by the page (side-effect module import) and the classic service worker.
// This database is deliberately separate from task documents and every backup.
(() => {
  class PushStore {
    constructor(scope, database = globalThis.indexedDB) { this.name = `ferroui.todo.push.v1:${scope}`; this.database = database; this.opened = null; }
    open() {
      if (!this.database) return Promise.reject(new Error("Private reminder storage is unavailable in this browser."));
      if (this.opened) return this.opened.promise;
      const attempt = { abandoned: false, promise: null }; this.opened = attempt;
      attempt.promise = new Promise((resolve, reject) => {
        let request;
        const discard = () => { if (this.opened === attempt) this.opened = null; };
        const fail = error => { attempt.abandoned = true; discard(); reject(error); };
        try { request = this.database.open(this.name, 1); } catch (error) { fail(error); return; }
        request.onupgradeneeded = () => {
          if (attempt.abandoned) { request.transaction.abort(); return; }
          request.result.createObjectStore("state");
          request.result.createObjectStore("receipts");
        };
        request.onerror = () => fail(request.error);
        request.onblocked = () => fail(new Error("Close older To Do tabs to update reminder storage."));
        request.onsuccess = () => {
          const database = request.result;
          if (attempt.abandoned) { database.close(); return; }
          database.onversionchange = () => { database.close(); discard(); };
          database.onclose = discard;
          resolve(database);
        };
      });
      return attempt.promise;
    }
    async transaction(stores, mode, operation) {
      const database = await this.open();
      return new Promise((resolve, reject) => {
        const transaction = database.transaction(stores, mode);
        let result, failure;
        const done = value => { result = value; };
        const guard = handler => event => { try { handler(event.target.result); } catch (error) { failure = error; transaction.abort(); } };
        transaction.oncomplete = () => resolve(result);
        transaction.onerror = transaction.onabort = () => reject(failure || transaction.error || new Error("Reminder storage failed."));
        try { operation(transaction, done, guard); } catch (error) { failure = error; transaction.abort(); }
      });
    }
    read() {
      return this.transaction(["state"], "readonly", (transaction, done, guard) => {
        transaction.objectStore("state").get("installation").onsuccess = guard(value => done(value || null));
      });
    }
    update(change) {
      return this.transaction(["state"], "readwrite", (transaction, done, guard) => {
        const store = transaction.objectStore("state");
        store.get("installation").onsuccess = guard(value => {
          const next = change(value || null);
          if (next instanceof Promise) throw new Error("Reminder storage updates must be synchronous.");
          if (next === null) store.delete("installation"); else store.put(next, "installation");
          done(next);
        });
      });
    }
    claim({ installation, workspace, id, occurrence, taskId, date, minutes, now, until, requirePlan = true }) {
      const claim = globalThis.crypto.randomUUID();
      return this.transaction(["state", "receipts"], "readwrite", (transaction, done, guard) => {
        transaction.objectStore("state").get("installation").onsuccess = guard(state => {
          if (!state?.enabled || state.id !== installation || state.workspace !== workspace || (requirePlan && state.expiresAt <= now)) return done(null);
          const job = state.plan?.[id];
          if (requirePlan && (!job || job.at > now + 30_000 || job.expiresAt <= now || job.occurrence !== occurrence)) return done(null);
          const receipts = transaction.objectStore("receipts");
          receipts.get(occurrence).onsuccess = guard(receipt => {
            if (receipt?.deliveredAt != null || (receipt?.claim && receipt.claim.until > now)) return done(null);
            receipts.put({ workspace, taskId, date, minutes, until: Math.max(until, now + 86_400_000), claim: { id: claim, until: now + 30_000 } }, occurrence);
            done({ claim, job });
          });
        });
      });
    }
    finish(occurrence, claim, now, success) {
      return this.transaction(["receipts"], "readwrite", (transaction, done, guard) => {
        const store = transaction.objectStore("receipts");
        store.get(occurrence).onsuccess = guard(value => {
          if (value?.claim?.id === claim) {
            if (success) store.put({ ...value, claim: null, deliveredAt: now }, occurrence);
            else store.delete(occurrence);
          }
          done();
        });
      });
    }
    prune(now, active) {
      return this.transaction(["receipts"], "readwrite", (transaction, done, guard) => {
        transaction.objectStore("receipts").openCursor().onsuccess = guard(cursor => {
          if (!cursor) return done();
          const receipt = cursor.value;
          if (receipt.deliveredAt == null && receipt.until <= now) cursor.delete();
          else if (active && receipt.workspace === active.workspace) {
            const reminder = active.reminders[receipt.taskId];
            if (reminder?.date === receipt.date && reminder?.minutes_after_midnight === receipt.minutes) {
              if (receipt.obsoleteAt != null) cursor.update({ ...receipt, obsoleteAt: null });
            } else if (receipt.obsoleteAt != null && receipt.obsoleteAt + 7 * 86_400_000 <= now) cursor.delete();
            else if (receipt.obsoleteAt == null) cursor.update({ ...receipt, obsoleteAt: now });
          }
          cursor.continue();
        });
      });
    }
    resetReceipts() {
      return this.transaction(["receipts"], "readwrite", transaction => transaction.objectStore("receipts").clear());
    }
  }
  globalThis.TodoPushStore = PushStore;
})();
