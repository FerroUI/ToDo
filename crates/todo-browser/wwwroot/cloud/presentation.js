import { assertDocument, commit, loadMetadata } from "./metadata.js";
import { createPresentationStore, clearPresentationCache } from "./presentation-store.js";
import { PRESENTATION_KIND, mergeRecords, validateReplica, validDevice, same } from "./presentation-format.js";
import { assignGroupIdentities, projectPresentation, recordLocalChanges, applyPresentation } from "./presentation-model.js";

const PREFIX = "ferroui.todo.presentation.v1.";
const lastReads = new Map();
const store = createPresentationStore();

export function forgetPresentationMappings() {
  const keys = [];
  for (let index = 0; index < localStorage.length; index++) {
    const key = localStorage.key(index);
    if (key?.startsWith(PREFIX)) keys.push(key);
  }
  for (const key of keys) localStorage.removeItem(key);
  clearPresentationCache(); lastReads.clear();
}

export function resetPresentationSession() { clearPresentationCache(); lastReads.clear(); }

export function createPresentationSync({ storage = () => localStorage, remote = store, now = () => Date.now(), uuid = () => crypto.randomUUID() } = {}) {
  return async function synchronize(runtime, account, { force = false } = {}) {
    const expected = runtime.exportData(), document = JSON.parse(expected);
    const { meta: graph } = loadMetadata(account);
    const key = `${PREFIX}${account}`;
    const raw = storage().getItem(key);
    const metadata = raw ? JSON.parse(raw) : { version: 1, device: uuid(), groups: {}, baseline: {}, records: {} };
    if (metadata.version !== 1 || !validDevice(metadata.device) || !metadata.groups || !metadata.baseline || !metadata.records) {
      throw new Error("The local device settings metadata is unreadable.");
    }
    validateReplica({ kind: PRESENTATION_KIND, version: 1, device: metadata.device, records: metadata.records });
    if (metadata.pending) {
      if (!metadata.pending.local || typeof metadata.pending.local !== "object" || Array.isArray(metadata.pending.local)) throw new Error("Pending device settings are unreadable.");
      validateReplica({ kind: PRESENTATION_KIND, version: 1, device: metadata.device, records: metadata.pending.records });
    }
    metadata.groups = assignGroupIdentities(document, metadata.groups, uuid);
    const local = projectPresentation(document, graph, metadata.groups);
    const readKey = `${account}|${metadata.device}`, previous = lastReads.get(readKey);
    if (!force && !metadata.pending && previous && now() - previous < 60_000 && same(local, metadata.baseline)) return false;
    // Persist identities before any remote write so a lost response cannot
    // create a second replica or duplicate locally created list groups.
    storage().setItem(key, JSON.stringify(metadata));
    const replicas = await remote.read(account);
    assertDocument(runtime, expected);
    const merged = mergeRecords(metadata.records, metadata.pending?.records, ...replicas.map(replica => validateReplica(replica).records));
    const update = recordLocalChanges(local, metadata.pending?.local || metadata.baseline, merged, metadata.device);
    const applied = applyPresentation(document, graph, metadata.groups, update.records);
    const prepared = runtime.prepareCloudData(JSON.stringify(applied.draft));
    const draft = JSON.parse(prepared);
    const replica = { kind: PRESENTATION_KIND, version: 1, device: metadata.device, records: update.records };
    validateReplica(replica);
    assertDocument(runtime, expected);
    if (update.changed || metadata.pending) {
      storage().setItem(key, JSON.stringify({ ...metadata, pending: { local, records: update.records } }));
      await remote.write(account, replica);
    }
    assertDocument(runtime, expected);
    const next = { ...metadata, pending: undefined, groups: applied.groups, records: update.records,
      baseline: projectPresentation(draft, graph, applied.groups) };
    const changed = commit(runtime, expected, draft, key, next);
    lastReads.set(readKey, now());
    return changed;
  };
}

export const synchronizePresentation = createPresentationSync();
