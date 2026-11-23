import { randomBytes } from "node:crypto";
import { DAY } from "./config.mjs";
import { HttpError, hash, equalSecret, token, exactKeys, subscription, applicationScope } from "./security.mjs";

function revision(value) {
  if (typeof value !== "string" || !/^"[0-9]{1,16}"$/.test(value) || !Number.isSafeInteger(Number(value.slice(1, -1)))) {
    throw new HttpError(428, "Send the current revision in If-Match.");
  }
  return Number(value.slice(1, -1));
}

export class Schedules {
  constructor(store, config, now = Date.now) { this.store = store; this.config = config; this.now = now; }
  authenticated(state, id, capability, origin) {
    const item = state.installations[id];
    if (!token(id, 22) || !token(capability) || !item || item.origin !== origin || !equalSecret(capability, item.authHash)) {
      throw new HttpError(401, "Invalid installation credentials.");
    }
    return item;
  }
  view(item) {
    return { revision: item.revision, expiresAt: item.expiresAt, subscriptionActive: !!item.subscription,
      scheduled: Object.values(item.jobs).filter(job => !job.deliveredAt && !job.failedAt && job.expiresAt > this.now()).length };
  }
  async enroll(body, origin) {
    exactKeys(body, ["id", "capability", "subscription", "scope"]);
    if (!token(body.id, 22) || !token(body.capability)) throw new HttpError(400, "Generate a new random installation ID and 32-byte capability.");
    const normalized = subscription(body.subscription, this.config), scope = applicationScope(body.scope, origin), now = this.now();
    if (normalized.expirationTime != null && normalized.expirationTime <= now) throw new HttpError(400, "The browser subscription has already expired.");
    return this.store.transact(state => {
      const existing = state.installations[body.id];
      if (existing) {
        if (existing.origin !== origin || existing.scope !== scope || !equalSecret(body.capability, existing.authHash)
          || JSON.stringify(existing.subscription) !== JSON.stringify(normalized)) throw new HttpError(409, "Installation ID is already in use.");
        return this.view(existing); // Idempotent retry after a lost response.
      }
      for (const [id, item] of Object.entries(state.installations)) if (item.expiresAt + 7 * DAY <= now) delete state.installations[id];
      if (Object.keys(state.installations).length >= this.config.maxInstallations) throw new HttpError(503, "Installation quota reached.");
      if (Object.values(state.installations).some(item => item.subscription?.endpoint === normalized.endpoint)) {
        throw new HttpError(409, "This browser subscription is already enrolled. Disable the old installation first.");
      }
      const item = { authHash: hash(body.capability), origin, scope, subscription: normalized, revision: 0,
        expiresAt: Math.min(now + this.config.leaseMs, normalized.expirationTime ?? Infinity), createdAt: now, updatedAt: now, jobs: {} };
      state.installations[body.id] = item;
      return this.view(item);
    });
  }
  get(id, capability, origin) { return this.view(this.authenticated(this.store.snapshot(), id, capability, origin)); }
  async update(id, capability, origin, match, body) {
    exactKeys(body, ["workspace", "jobs", "subscription"]);
    const expected = revision(match), now = this.now();
    if (!token(body.workspace) || !Array.isArray(body.jobs) || body.jobs.length > this.config.maxJobs) throw new HttpError(400, "Invalid workspace or reminder count.");
    const jobs = {}, subscriptionValue = body.subscription ? subscription(body.subscription, this.config) : null;
    if (subscriptionValue?.expirationTime != null && subscriptionValue.expirationTime <= now) throw new HttpError(400, "The browser subscription has already expired.");
    for (const value of body.jobs) {
      exactKeys(value, ["id", "at", "expiresAt", "title"]);
      if (!token(value.id) || Object.hasOwn(jobs, value.id) || !Number.isSafeInteger(value.at) || value.at < now - this.config.expiryMs
        || value.at > now + this.config.horizonMs || !Number.isSafeInteger(value.expiresAt) || value.expiresAt <= value.at
        || value.expiresAt > value.at + this.config.expiryMs || value.expiresAt <= now || typeof value.title !== "string"
        || !value.title.trim() || value.title.length > 200 || /[\u0000-\u001f\u007f]/.test(value.title)) {
        throw new HttpError(400, "A reminder has an invalid ID, title, delivery time or expiry.");
      }
      jobs[value.id] = { id: value.id, at: value.at, expiresAt: value.expiresAt, title: value.title, workspace: body.workspace,
        attempts: 0, nextAt: value.at, claim: null };
    }
    return this.store.transact(state => {
      const item = this.authenticated(state, id, capability, origin);
      if (item.revision !== expected) throw new HttpError(409, "The schedule changed. Read it and rebuild from the active workspace.", { revision: item.revision });
      if (item.revision >= Number.MAX_SAFE_INTEGER) throw new HttpError(409, "Installation revision exhausted. Re-enroll this browser.");
      if (subscriptionValue) {
        if (Object.entries(state.installations).some(([otherId, other]) => otherId !== id && other.subscription?.endpoint === subscriptionValue.endpoint)) {
          throw new HttpError(409, "Subscription is already attached to another installation.");
        }
        item.subscription = subscriptionValue;
      }
      if (!item.subscription) throw new HttpError(410, "Subscription expired. Refresh it in the app.");
      if (item.subscription.expirationTime != null && item.subscription.expirationTime <= now) throw new HttpError(410, "Subscription expired. Refresh it in the app.");
      for (const [key, job] of Object.entries(jobs)) {
        const old = item.jobs[key];
        if (!old) continue;
        if (old.at !== job.at || old.workspace !== job.workspace || old.expiresAt !== job.expiresAt) {
          throw new HttpError(409, "A reminder ID cannot be reused for a different occurrence.");
        }
        // Renaming changes pending text without creating another occurrence.
        // A message already in flight is rendered from the worker's local plan.
        jobs[key] = { ...old, title: job.title };
      }
      item.jobs = jobs; item.workspace = body.workspace; item.revision++;
      item.updatedAt = now; item.expiresAt = Math.min(now + this.config.leaseMs, item.subscription.expirationTime ?? Infinity);
      return this.view(item);
    });
  }
  async remove(id, capability, origin) {
    return this.store.transact(state => {
      this.authenticated(state, id, capability, origin);
      delete state.installations[id];
    });
  }
}

export function deliveryClaim() { return randomBytes(16).toString("base64url"); }
