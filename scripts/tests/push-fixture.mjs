import { PushClient } from "../../crates/todo-browser/wwwroot/push-client.js";
import { publicKey } from "../../crates/todo-browser/wwwroot/push-api.js";
import { randomToken } from "../../crates/todo-browser/wwwroot/push-plan.js";

export const key = Buffer.concat([Buffer.from([4]), Buffer.alloc(64, 1)]).toString("base64url");
export const config = { version: 1, publicKey: key, enrollmentRequired: true, maxJobs: 200,
  horizonMs: 366 * 86_400_000, expiryMs: 3600_000, leaseMs: 366 * 86_400_000 };
export const scope = "https://tasks.example.test/ToDo/";
export const task = (changes = {}) => ({ id: 3, title: "Remember", completed: false,
  reminder: { date: "2026-11-23", minutes_after_midnight: 720, notified: false }, ...changes });

// Protocol fixture: browser storage/network capabilities are injected, while
// production document extraction, hashing, CAS retry and notification gates run.
export class PrivateStore {
  constructor() { this.state = null; this.receipts = new Map(); this.pending = Promise.resolve(); }
  async read() { return structuredClone(this.state); }
  update(change) {
    const next = this.pending.then(() => { this.state = change(structuredClone(this.state)); return structuredClone(this.state); });
    this.pending = next.catch(() => {}); return next;
  }
  async resetReceipts() { this.receipts.clear(); }
  async prune() {}
  async claim(input) {
    let result = null;
    await this.update(state => {
      if (!state?.enabled || state.id !== input.installation || state.workspace !== input.workspace || (input.requirePlan !== false && state.expiresAt <= input.now)) return state;
      const job = state.plan?.[input.id];
      if (input.requirePlan !== false && (!job || job.expiresAt <= input.now || job.at > input.now + 30_000)) return state;
      const previous = this.receipts.get(input.occurrence);
      if (previous?.deliveredAt != null || (previous?.claim && previous.claim.until > input.now)) return state;
      const claim = randomToken(16); this.receipts.set(input.occurrence, { claim: { id: claim, until: input.now + 30_000 }, until: input.until });
      result = { claim, job }; return state;
    });
    return result;
  }
  async finish(id, claim, now, success) {
    if (this.receipts.get(id)?.claim?.id !== claim) return;
    if (success) this.receipts.set(id, { deliveredAt: now }); else this.receipts.delete(id);
  }
}

export function fixture({ tasks = [task()], supported = true } = {}) {
  let now = new Date("2026-11-23T12:00:00").getTime(), document = { tasks }, account = null, blocked = false, permission = "granted";
  let installed = null, permissionRequests = 0, subscriptions = 0, unsubscribed = 0, offline = false, afterGet;
  const store = new PrivateStore(), states = [], requests = [], remote = new Map();
  const subscription = () => ({ options: { applicationServerKey: publicKey(key).buffer },
    toJSON: () => ({ endpoint: `https://fcm.googleapis.com/fcm/send/subscription-${subscriptions}`, expirationTime: null,
      keys: { p256dh: key, auth: Buffer.alloc(16, 2).toString("base64url") } }),
    unsubscribe: async () => { unsubscribed++; installed = null; return true; } });
  const registration = { scope, active: {}, pushManager: {
    getSubscription: async () => installed,
    subscribe: async options => { if (!options.userVisibleOnly) throw new Error("Visible notification required"); subscriptions++; return installed = subscription(); },
  } };
  const fetcher = async (url, options = {}) => {
    const path = new URL(url).pathname, body = options.body ? JSON.parse(options.body) : null;
    requests.push({ path, ...options, body });
    if (offline) throw new Error("Offline");
    const respond = (value, status = 200) => new Response(value === null ? null : JSON.stringify(value), { status });
    if (path === "/v1/config") return respond(config);
    if (path === "/v1/installations" && options.method === "POST") {
      const previous = remote.get(body.id);
      if (previous && previous.capability !== body.capability) return respond({ error: "Conflict" }, 409);
      remote.set(body.id, previous || { revision: 0, capability: body.capability, subscriptionActive: true, expiresAt: now + config.leaseMs });
      return respond(remote.get(body.id), 201);
    }
    const id = path.split("/").at(-1), item = remote.get(id);
    if (!item || options.headers.authorization !== `Bearer ${item.capability}`) return respond({ error: "Invalid installation credentials." }, 401);
    if (options.method === "DELETE") { remote.delete(id); return respond(null, 204); }
    if (options.method === "GET") {
      const result = structuredClone(item); if (afterGet) await afterGet(item); return respond(result);
    }
    if (options.method === "PUT") {
      if (options.headers["if-match"] !== `"${item.revision}"`) return respond({ error: "Schedule changed", revision: item.revision }, 409);
      Object.assign(item, { revision: item.revision + 1, workspace: body.workspace, jobs: body.jobs, expiresAt: now + config.leaseMs });
      if (body.subscription) item.subscriptionActive = true;
      return respond(item);
    }
    return respond({ error: "Unknown path" }, 404);
  };
  const client = new PushClient({ scope, store, supported, reason: "Unsupported test host", now: () => now, fetcher,
    readWorkspace: () => ({ account, blocked, json: JSON.stringify(document) }), registration: async () => registration,
    permission: () => permission, requestPermission: async () => { permissionRequests++; return permission; }, onState: state => states.push(state),
  });
  return { client, store, states, requests, remote, registration, now: () => now, advance: ms => { now += ms; },
    setDocument: value => { document = structuredClone(value); }, setAccount: value => { account = value; },
    setBlocked: value => { blocked = value; }, setPermission: value => { permission = value; },
    setOffline: value => { offline = value; }, afterGet: callback => { afterGet = callback; },
    subscriptions: () => subscriptions, unsubscribed: () => unsubscribed, permissionRequests: () => permissionRequests,
    enable: () => client.enable({ service_url: "https://reminders.example.test/", public_key: key, enrollment_key: "x".repeat(43) }),
  };
}
