import * as auth from "./auth.js";
import { collection } from "./client.js";
import { normalizeCategories } from "./category-colors.js";
import { requireCloudService } from "./environment.js";

export const CATEGORY_SCOPES = Object.freeze(["MailboxSettings.Read"]);
const PREFIX = "ferroui.todo.microsoft.categories.";
const FRESH_FOR = 5 * 60_000;

export function createCategoryCatalog({ collection, accountKey, environment, storage, now = Date.now }) {
  const pending = new Map(), epochs = new Map();
  let generation = 0;
  const key = account => `${PREFIX}${encodeURIComponent(account)}`;
  const requireAccount = account => {
    if (!account || account !== accountKey()) throw Object.assign(new Error("The Microsoft account changed while reading Outlook categories."), { code: "account_changed" });
    requireCloudService(environment(), "categories");
  };
  const forgetCached = account => { try { storage().removeItem(key(account)); } catch { /* Optional cache. */ } };

  function cached(account) {
    if (!account || account !== accountKey()) return null;
    try {
      const json = storage().getItem(key(account));
      if (!json) return null;
      if (json.length > 1024 * 1024) throw new Error("Excessive category cache");
      const value = JSON.parse(json);
      if (value.version !== 1 || value.account !== account || !Number.isFinite(value.at) || value.at < 0 || value.at > now() + 60_000) throw new Error("Invalid category cache");
      return { at: value.at, categories: normalizeCategories(value.items) };
    } catch { forgetCached(account); return null; }
  }

  async function read(account, { force = false } = {}) {
    requireAccount(account);
    const saved = cached(account);
    if (!force && saved && now() - saved.at < FRESH_FOR) return saved.categories;
    if (pending.has(account)) return pending.get(account);
    const expected = generation, epoch = epochs.get(account) || 0;
    const operation = (async () => {
      const items = await collection("me/outlook/masterCategories?$select=id,displayName,color", { scopes: CATEGORY_SCOPES });
      requireAccount(account);
      if (generation !== expected || (epochs.get(account) || 0) !== epoch) throw Object.assign(new Error("Outlook categories were disconnected while the request was pending."), { code: "account_changed" });
      const categories = normalizeCategories(items);
      const cache = { version: 1, account, at: now(), items: items.map(item => ({ id: item.id, displayName: item.displayName, color: item.color })) };
      try {
        const json = JSON.stringify(cache);
        if (json.length <= 1024 * 1024) storage().setItem(key(account), json);
      } catch { /* Storage pressure must not discard a successful catalogue read. */ }
      return categories;
    })();
    pending.set(account, operation);
    try { return await operation; }
    finally { if (pending.get(account) === operation) pending.delete(account); }
  }

  function forget(account) {
    if (account) {
      epochs.set(account, (epochs.get(account) || 0) + 1);
      pending.delete(account); forgetCached(account);
      return;
    }
    generation++; pending.clear(); epochs.clear();
    try {
      const cache = storage();
      for (let i = cache.length - 1; i >= 0; i--) {
        const name = cache.key(i);
        if (name?.startsWith(PREFIX)) cache.removeItem(name);
      }
    } catch { /* Optional cache. */ }
  }

  return { read, cached: account => cached(account)?.categories || [], forget };
}

const catalogue = createCategoryCatalog({
  collection, accountKey: () => auth.accountKey(), environment: () => auth.serviceEnvironment(), storage: () => localStorage,
});
export const readOutlookCategories = (account, options) => catalogue.read(account, options);
export const cachedOutlookCategories = account => catalogue.cached(account);
export const forgetOutlookCategories = account => catalogue.forget(account);
