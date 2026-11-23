import * as auth from "./auth.js";
import { graph, segment } from "./client.js";
import { readOutlookCategories, forgetOutlookCategories } from "./categories.js";
import { requireCloudService } from "./environment.js";

export const CATEGORY_WRITE_SCOPES = Object.freeze(["MailboxSettings.ReadWrite"]);
export const categoryManagementActions = Object.freeze([
  "createOutlookCategory", "updateOutlookCategory", "deleteOutlookCategory",
]);

const fail = message => { throw new Error(message); };
const validId = value => typeof value === "string" && value.length > 0 && value.length <= 1024 && !/[\u0000-\u001f\u007f]/u.test(value);
export function categoryPreset(value) {
  if (typeof value !== "string" || !/^(?:none|preset(?:[0-9]|1[0-9]|2[0-4]))$/.test(value)) fail("Choose an Outlook category color.");
  return value;
}
export function categoryName(value) {
  const name = typeof value === "string" ? value.trim() : "";
  if (!name || [...name].length > 255 || /[\u0000-\u001f\u007f]/u.test(name)) fail("Enter a category name of 1–255 characters without line breaks.");
  return name;
}

// The host serializes mutations with account changes. This guard also protects
// independently imported callers and delayed responses from an old session.
export function createCategoryManager({ graph, read, forget, accountKey, environment }) {
  const guard = account => {
    if (!account || account !== accountKey()) throw Object.assign(new Error("The Microsoft account changed while editing Outlook categories."), { code: "account_changed" });
    requireCloudService(environment(), "categories");
  };
  const reload = async account => {
    guard(account);
    forget(account);
    const categories = await read(account, { force: true });
    guard(account);
    return categories;
  };
  return async (account, action) => {
    if (!categoryManagementActions.includes(action?.kind)) fail("Unknown Outlook category action.");
    guard(account);
    const creating = action.kind === "createOutlookCategory";
    const deleting = action.kind === "deleteOutlookCategory";
    const name = creating ? categoryName(action.name) : null;
    const color = deleting ? null : categoryPreset(action.color);
    if (!creating && !validId(action.id)) fail("Select an Outlook category.");
    if (action.expected_color != null) categoryPreset(action.expected_color);
    const before = await reload(account);
    const existing = creating ? before.find(item => item.name.toLowerCase() === name.toLowerCase()) : before.find(item => item.id === action.id);
    if (creating && existing) fail("An Outlook category with this name already exists.");
    if (!creating && !existing) {
      if (deleting) return before;
      fail("This category was removed in Outlook. Refresh the catalogue and choose another category.");
    }
    if (!creating && !deleting && action.expected_color != null && existing.preset !== action.expected_color) {
      fail("This category color changed in Outlook. Refresh the catalogue before changing it again.");
    }
    if (!creating && !deleting && existing.preset === color) return before;
    guard(account);
    try {
      await graph(creating ? "me/outlook/masterCategories" : `me/outlook/masterCategories/${segment(action.id)}`,
        creating ? "POST" : deleting ? "DELETE" : "PATCH",
        creating ? { displayName: name, color } : deleting ? undefined : { color }, undefined,
        { scopes: CATEGORY_WRITE_SCOPES });
    } catch (error) {
      // A dropped POST response may already have created the category. Read the
      // unique name before allowing a retry; never blindly replay creation.
      guard(account);
      if (creating && (!error.status || error.status === 409 || error.status >= 500)) {
        try {
          const recovered = await reload(account);
          if (recovered.some(item => item.name.toLowerCase() === name.toLowerCase() && item.preset === color)) return recovered;
        } catch (recoveryError) { if (recoveryError.code === "account_changed") throw recoveryError; }
      }
      forget(account);
      throw error;
    }
    guard(account);
    try { return await reload(account); }
    catch (error) {
      if (error.code === "account_changed") throw error;
      throw new Error(`The category change was saved, but the catalogue could not be refreshed. ${error.message}`);
    }
  };
}

export const manageOutlookCategory = createCategoryManager({
  graph, read: readOutlookCategories, forget: forgetOutlookCategories,
  accountKey: () => auth.accountKey(), environment: () => auth.serviceEnvironment(),
});
