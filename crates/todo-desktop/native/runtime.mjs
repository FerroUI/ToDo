import { ui } from "./bridge.mjs";

let expectedImport;
export const runtime = {
  exportData: () => ui("exportData"),
  prepareCloudData: json => ui("prepareCloudData", { json }),
  replaceFromCloud: (json, expected) => ui("replaceFromCloud", { json, expected }),
  validateImportData: json => ui("validateImportData", { json }),
  importData: json => ui("importData", { json, expected: expectedImport }),
  cloudState: (connected, label) => ui("cloudState", { connected, label }),
  cloudCapabilities: (supported, configured, workAccount) => ui("cloudCapabilities", { supported, configured, workAccount }),
  cloudHint: message => ui("cloudHint", { message }),
  hostError: message => ui("hostError", { message }),
  hostStatus: message => ui("hostStatus", { message }),
  outlookCategories: json => ui("outlookCategories", { json }),
  setIntegrationPreference: (feature, enabled) => ui("setIntegrationPreference", { feature, enabled }),
  flushEdits: () => ui("flushEdits"),
};
export function beginImport(expected) { expectedImport = expected; }
export function endImport() { expectedImport = undefined; }
