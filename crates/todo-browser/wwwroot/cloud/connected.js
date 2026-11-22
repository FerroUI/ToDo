import * as auth from "./auth.js";
import { readOutlookCategories, cachedOutlookCategories, forgetOutlookCategories, CATEGORY_SCOPES } from "./categories.js";
import { synchronizePresentation, forgetPresentationMappings, resetPresentationSession } from "./presentation.js";
import { PRESENTATION_SCOPE } from "./presentation-format.js";
import { requireCloudService } from "./environment.js";
import { requiresSyncReview } from "./automatic.js";

const failures = new Map();
export const connectedActions = Object.freeze(["enableOutlookCategories", "refreshOutlookCategories", "disableOutlookCategories", "enablePresentationSync", "disablePresentationSync"]);

export function resetConnectedApps(runtime, { restore = false } = {}) {
  failures.clear(); forgetOutlookCategories(); runtime.outlookCategories("[]");
  if (restore) forgetPresentationMappings();
  else resetPresentationSession();
}

export function connectedPreferencesChanged() { failures.clear(); }

export function restoreConnectedApps(runtime, account, preferences) {
  if (preferences.outlook_categories_enabled) runtime.outlookCategories(JSON.stringify(cachedOutlookCategories(account)));
  else runtime.outlookCategories("[]");
}

export async function connectedAppRequest(runtime, kind) {
  if (!connectedActions.includes(kind)) return false;
  const account = auth.accountKey();
  if (!account) throw new Error("Connect your Microsoft account first.");
  const categories = kind.includes("OutlookCategories");
  const preference = categories ? "outlook_categories_enabled" : "presentation_sync_enabled";
  const service = categories ? "categories" : "presentation";
  if (kind.startsWith("disable")) {
    runtime.setIntegrationPreference(preference, false);
    if (categories) { forgetOutlookCategories(account); runtime.outlookCategories("[]"); }
    failures.delete(service);
    return true;
  }
  requireCloudService(auth.serviceEnvironment(), service);
  failures.delete(service);
  const previous = JSON.parse(runtime.exportData()).settings[preference] === true;
  runtime.setIntegrationPreference(preference, true);
  try {
    if (kind === "refreshOutlookCategories") {
      try { runtime.outlookCategories(JSON.stringify(await readOutlookCategories(account, { force: true }))); return true; }
      catch (error) { if (!error.requiresInteraction) throw error; }
    }
    await auth.requestScopes(categories ? CATEGORY_SCOPES : PRESENTATION_SCOPE);
  } catch (error) {
    // A rejected/blocked consent request must not pretend this service is active.
    runtime.setIntegrationPreference(preference, previous);
    throw error;
  }
  return true;
}

export async function synchronizeConnectedApps(runtime, account, preferences, { force = false } = {}) {
  let changed = false;
  const issues = [];
  const run = async (service, label, enabled, perform) => {
    if (!enabled) return;
    const previous = failures.get(service);
    if (!force && previous && Date.now() < previous.retryAt) { issues.push(previous.message); return; }
    try {
      requireCloudService(auth.serviceEnvironment(), service);
      changed = (await perform()) || changed;
      failures.delete(service);
    } catch (error) {
      const message = `${label}: ${error.message}`;
      const permanent = requiresSyncReview(error) || /invalid|unsupported|too many|exceed|unreadable|same revision|incomplete/i.test(error.message || "");
      failures.set(service, { message, retryAt: permanent ? Infinity : Date.now() + 60_000 });
      issues.push(message);
      if (force || previous?.message !== message) runtime.hostError(message);
    }
  };
  await run("categories", "Outlook categories", preferences.outlook_categories_enabled, async () => {
    runtime.outlookCategories(JSON.stringify(await readOutlookCategories(account, { force })));
    return false;
  });
  await run("presentation", "Device settings sync", preferences.presentation_sync_enabled,
    () => synchronizePresentation(runtime, account, { force }));
  return { changed, issues };
}
