import { ui } from "./bridge.mjs";
import { commitWorkspaceImport } from "../web/cloud/workspaces.js";

let expectedImport, beforeImportCommit;
export const runtime = {
  exportData: () => ui("exportData"),
  prepareCloudData: json => ui("prepareCloudData", { json }),
  prepareImportData: json => ui("prepareImportData", { json }),
  replaceFromCloud: (json, expected) => ui("replaceFromCloud", { json, expected }),
  emptyAccountData: () => ui("emptyAccountData"),
  replaceAccountData: (json, expected) => ui("replaceAccountData", { json, expected }),
  validateImportData: json => ui("validateImportData", { json }),
  importData: json => commitWorkspaceImport(runtime, json,
    prepared => ui("importData", { json: prepared, expected: expectedImport }), { beforeCommit: () => beforeImportCommit?.() }),
  cloudState: (connected, label) => ui("cloudState", { connected, label }),
  cloudCapabilities: (supported, configured, workAccount) => ui("cloudCapabilities", { supported, configured, workAccount }),
  cloudHint: message => ui("cloudHint", { message }),
  hostError: message => ui("hostError", { message }),
  hostStatus: message => ui("hostStatus", { message }),
  outlookCategories: json => ui("outlookCategories", { json }),
  setIntegrationPreference: (feature, enabled) => ui("setIntegrationPreference", { feature, enabled }),
  flushEdits: () => ui("flushEdits"),
};
export function beginImport(expected, prepareCommit) { expectedImport = expected; beforeImportCommit = prepareCommit; }
export function endImport() { expectedImport = undefined; beforeImportCommit = undefined; }
