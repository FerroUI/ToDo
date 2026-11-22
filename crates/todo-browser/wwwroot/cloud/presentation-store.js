import { graph, collection, segment } from "./client.js";
import { serviceEnvironment } from "./auth.js";
import { MAX_PRESENTATION_BYTES, MAX_REPLICAS, PRESENTATION_SCOPE, validateReplica, validDevice } from "./presentation-format.js";
import { requireCloudService } from "./environment.js";
import { boundedBlob } from "./response.js";

const prefix = "ferroui-todo-presentation-";
const scopes = { scopes: PRESENTATION_SCOPE };
const caches = new Map();
const MAX_TOTAL_BYTES = 16 * 1024 * 1024;
let generation = 0;
export const clearPresentationCache = () => { generation++; caches.clear(); };
export const replicaName = device => {
  if (!validDevice(device)) throw new Error("Invalid device settings identity.");
  return `${prefix}${device}.json`;
};

export function presentationDownloadUrl(value, environment = serviceEnvironment()) {
  requireCloudService(environment, "presentation");
  const url = new URL(value);
  const host = url.hostname.toLowerCase();
  const suffixes = environment.id === "global"
    ? [".1drv.com", ".onedrive.com", ".storage.live.com", ".sharepoint.com", ".sharepoint-df.com"]
    : environment.id === "usgov" ? [".sharepoint.us"] : [".sharepoint-mil.us", ".dps.mil"];
  if (url.protocol !== "https:" || url.username || url.password || url.port || url.hash
    || !suffixes.some(suffix => host.endsWith(suffix))) throw new Error("OneDrive returned an unexpected device settings download URL.");
  return url;
}

export async function readPresentationResponse(response, { expectedSize } = {}) {
  if (!response.ok) throw new Error(`OneDrive device settings download returned ${response.status}.`);
  if (response.status !== 200 || response.headers.has("Content-Range")) {
    await response.body?.cancel().catch(() => {});
    throw new Error("OneDrive returned a partial device settings download. Sync again to read the complete file.");
  }
  const blob = await boundedBlob(response, MAX_PRESENTATION_BYTES);
  if (expectedSize !== undefined && blob.size !== expectedSize) throw new Error("OneDrive device settings size changed while downloading. Sync again to read the complete file.");
  const bytes = new Uint8Array(await blob.arrayBuffer());
  return validateReplica(JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes)));
}

export function createPresentationStore({ request = graph, list = collection, download = (...args) => fetch(...args), environment = serviceEnvironment } = {}) {
  const context = account => {
    if (typeof account !== "string" || !account || account.length > 4096) throw new Error("Connect your Microsoft account before synchronizing device settings.");
    const cloud = environment(); requireCloudService(cloud, "presentation");
    const epoch = generation;
    return { cloud, key: JSON.stringify([cloud.id, account]), assertCurrent() {
      if (generation !== epoch || environment().id !== cloud.id) throw Object.assign(new Error("The Microsoft connection changed while synchronizing device settings."), { code: "account_changed" });
    } };
  };
  const validId = value => typeof value === "string" && value.length > 0 && value.length <= 2048 && !/[\u0000-\u001f]/.test(value);
  const validEtag = value => typeof value === "string" && value.trim().length > 0 && value.length <= 8192;
  const validSize = value => Number.isSafeInteger(value) && value >= 0 && value <= MAX_PRESENTATION_BYTES;
  return {
    async read(account) {
      const current = context(account);
      const items = await list("me/drive/special/approot/children?$select=id,name,eTag,size,file", scopes);
      current.assertCurrent();
      if (!Array.isArray(items)) throw new Error("OneDrive returned an invalid app folder.");
      const files = items.filter(item => typeof item.name === "string" && item.name.startsWith(prefix) && item.name.endsWith(".json"));
      if (files.length > MAX_REPLICAS || files.reduce((size, item) => size + (Number(item.size) || 0), 0) > MAX_TOTAL_BYTES) {
        throw new Error("The OneDrive app folder contains too many device settings replicas. Keep an independent backup before removing unused device files.");
      }
      const cache = caches.get(current.key) || new Map();
      const next = new Map(), replicas = [], names = new Set();
      let total = 0;
      for (const item of files) {
        const device = item.name.slice(prefix.length, -5);
        if (!validDevice(device) || !item.file || !validId(item.id) || !validEtag(item.eTag) || !validSize(item.size)
          || next.has(item.id) || names.has(item.name.toLowerCase())) {
          throw new Error("OneDrive contains an invalid device settings file.");
        }
        names.add(item.name.toLowerCase());
        let stored = cache.get(item.id);
        if (!stored || stored.etag !== item.eTag || stored.size !== item.size) {
          const path = `me/drive/items/${segment(item.id)}`;
          const detail = await request(path, "GET", undefined, undefined, scopes);
          current.assertCurrent();
          if (detail?.id !== item.id || detail.name !== item.name || !detail.file || !validEtag(detail.eTag)
            || !validSize(detail.size)) throw new Error("The OneDrive device settings file changed identity.");
          if (total + detail.size > MAX_TOTAL_BYTES) throw new Error("OneDrive device settings replicas exceed the 16 MiB combined limit.");
          const url = presentationDownloadUrl(detail["@microsoft.graph.downloadUrl"], current.cloud);
          // The returned URL is already authorized. Graph tokens never leave Graph.
          const response = await download(url, { credentials: "omit", redirect: "error", cache: "no-store",
            referrerPolicy: "no-referrer", signal: AbortSignal.timeout(30_000) });
          const replica = await readPresentationResponse(response, { expectedSize: detail.size });
          if (replica.device !== device) throw new Error("OneDrive device settings do not match their device identity.");
          current.assertCurrent();
          const latest = await request(`${path}?$select=id,eTag`, "GET", undefined, undefined, scopes);
          if (latest?.id !== item.id || latest.eTag !== detail.eTag) throw new Error("Device settings changed while downloading. Sync again to read the complete revision.");
          stored = { etag: detail.eTag, size: detail.size, replica };
        }
        total += stored.size;
        if (total > MAX_TOTAL_BYTES) throw new Error("OneDrive device settings replicas exceed the 16 MiB combined limit.");
        next.set(item.id, stored); replicas.push(structuredClone(stored.replica));
      }
      current.assertCurrent();
      caches.set(current.key, next);
      return replicas;
    },
    async write(account, replica) {
      const current = context(account);
      validateReplica(replica);
      const name = replicaName(replica.device);
      const size = new TextEncoder().encode(JSON.stringify(replica)).length;
      // A stable device owns exactly one file. Devices never overwrite one
      // another's snapshots; field revisions merge them deterministically.
      const item = await request(`me/drive/special/approot:/${name}:/content`, "PUT", replica, undefined, scopes);
      current.assertCurrent();
      if (!validId(item?.id) || !validEtag(item.eTag) || item.name !== name || !item.file || item.size !== size) throw new Error("OneDrive did not acknowledge the device settings revision.");
      const cache = new Map(caches.get(current.key) || []);
      cache.set(item.id, { etag: item.eTag, size, replica: structuredClone(replica) });
      if (cache.size <= MAX_REPLICAS && [...cache.values()].reduce((total, item) => total + item.size, 0) <= MAX_TOTAL_BYTES) caches.set(current.key, cache);
      else caches.delete(current.key);
    },
  };
}
