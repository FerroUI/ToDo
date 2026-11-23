import { deliveryClaim } from "./schedules.mjs";
import { safeAgent, subscription as validateSubscription } from "./security.mjs";
import { sendEncrypted } from "./transport.mjs";

export function retryDelay(error, attempts, now) {
  const raw = error?.headers?.["retry-after"], seconds = Number(raw);
  const requested = raw == null ? 0 : Number.isFinite(seconds) ? seconds * 1000 : Date.parse(raw) - now;
  return Math.min(15 * 60_000, Math.max(5000 * 2 ** Math.min(attempts - 1, 8), Number.isFinite(requested) ? requested : 0));
}

/** Persistent claims survive process restarts. Push services provide at-least-
 * once delivery; the service worker's durable receipt makes display idempotent. */
export class DeliveryQueue {
  constructor(store, config, { now = Date.now, send = sendEncrypted, agent = safeAgent() } = {}) {
    this.store = store; this.config = config; this.now = now; this.send = send; this.agent = agent; this.running = null;
  }
  tick() {
    if (this.running) return this.running;
    this.running = this.run().finally(() => { this.running = null; });
    return this.running;
  }
  async run() {
    const now = this.now(), ready = [];
    // Avoid rewriting/fsyncing the data file on every idle clock tick.
    const snapshot = this.store.snapshot();
    const eligible = (item, job) => item.expiresAt > now && item.subscription && !job.deliveredAt && !job.failedAt
      && job.at <= now && job.nextAt <= now && job.expiresAt > now && (!job.claim || job.claim.until <= now);
    if (!Object.values(snapshot.installations).some(item => Object.values(item.jobs).some(job => eligible(item, job)))) return;
    await this.store.transact(state => {
      for (const [installation, item] of Object.entries(state.installations)) {
        for (const job of Object.values(item.jobs)) {
          if (ready.length >= 8) return;
          if (!eligible(item, job)) continue;
          job.claim = { id: deliveryClaim(), until: now + 60_000 }; job.attempts++;
          ready.push({ installation, job: structuredClone(job), subscription: structuredClone(item.subscription) });
        }
      }
    });
    await Promise.all(ready.map(delivery => this.deliver(delivery)));
  }
  async deliver({ installation, job, subscription }) {
    const current = this.store.snapshot().installations[installation];
    if (!current || current.jobs[job.id]?.claim?.id !== job.claim.id || current.subscription?.endpoint !== subscription.endpoint
      || current.expiresAt <= this.now() || job.expiresAt <= this.now()) return;
    let error;
    try {
      // Revalidate stored endpoints after configuration changes/restores.
      validateSubscription(subscription, this.config);
      const ttl = Math.floor((job.expiresAt - this.now()) / 1000);
      if (ttl < 1) return;
      const payload = JSON.stringify({ version: 1, installation, workspace: job.workspace, id: job.id,
        at: job.at, expiresAt: job.expiresAt, title: job.title });
      await this.send(subscription, payload, { vapidDetails: this.config.vapid, TTL: ttl, urgency: "high",
        topic: job.id.slice(0, 32), timeout: this.config.requestTimeoutMs, contentEncoding: "aes128gcm", agent: this.agent });
    } catch (caught) { error = caught; }
    await this.store.transact(state => {
      const item = state.installations[installation], stored = item?.jobs[job.id];
      if (!item || stored?.claim?.id !== job.claim.id) return; // Replaced/deleted during I/O.
      stored.claim = null;
      const now = this.now();
      if (!error) { stored.deliveredAt = now; return; }
      const status = error.statusCode ?? error.status;
      if (status === 404 || status === 410) {
        if (item.subscription?.endpoint === subscription.endpoint) { item.subscription = null; item.jobs = {}; item.revision++; }
        return;
      }
      const retryable = status == null || status === 408 || status === 429 || status >= 500;
      const next = now + retryDelay(error, stored.attempts, now);
      if (retryable && stored.attempts < this.config.maxAttempts && next < stored.expiresAt) stored.nextAt = next;
      else stored.failedAt = now;
      // Never persist response bodies: a provider can echo subscription data.
    });
  }
  async close() { await this.running; this.agent.destroy(); }
}
