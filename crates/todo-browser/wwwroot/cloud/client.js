import * as auth from "./auth.js";
import { createCloudReader } from "./delta.js";
import { cloudEnvironment, requireCloudService } from "./environment.js";
import { boundedBlob } from "./response.js";

let mutationRevision = 0;
export const cloudRevision = () => mutationRevision;
export const segment = value => encodeURIComponent(value);
export const listPath = id => `me/todo/lists/${segment(id)}`;
export const taskPath = (listId, id) => `${listPath(listId)}/tasks/${segment(id)}`;

export function graphUrl(path) {
  const environment = auth.serviceEnvironment?.() || cloudEnvironment();
  const url = new URL(path, `${environment.graph}/v1.0/`);
  if (url.origin !== environment.graph || !url.pathname.startsWith("/v1.0/") || url.username || url.password || url.hash) {
    throw new Error("Microsoft returned an unexpected continuation URL.");
  }
  if (/\/(?:todo|planner)(?:\/|$)/i.test(url.pathname)) requireCloudService(environment, /\/planner(?:\/|$)/i.test(url.pathname) ? "planner" : "todo");
  if (/\/outlook\/masterCategories(?:\/|$)/i.test(url.pathname)) requireCloudService(environment, "categories");
  return url;
}

async function request(path, method, body, etag, attempt = 0, renew = false, rawHeaders, options = {}) {
  const url = graphUrl(path);
  const headers = { Authorization: `Bearer ${await auth.accessToken(renew, options.scopes)}`, Accept: "application/json" };
  if (body !== undefined) headers["Content-Type"] = "application/json";
  if (rawHeaders) Object.assign(headers, rawHeaders);
  if (etag) headers["If-Match"] = etag;
  const response = await fetch(url, {
    method, headers, body: rawHeaders ? body : body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(30_000), cache: "no-store", credentials: "omit", redirect: "error",
  });
  // A failed POST may already have been committed. Creation recovery owns that
  // uncertainty; only an explicit authorization/throttling rejection is replayed.
  const retry = response.status === 401 || response.status === 429
    || (response.status === 503 && method !== "POST" && (!rawHeaders || method === "GET"));
  if (retry && attempt < 2) {
    const header = response.headers.get("Retry-After");
    const delay = Number(header) || Math.max(0, (Date.parse(header) - Date.now()) / 1000) || 2 ** attempt;
    await new Promise(resolve => setTimeout(resolve, Math.min(20, Math.max(1, delay)) * 1000));
    return request(path, method, body, etag, attempt + 1, response.status === 401, rawHeaders, options);
  }
  if (response.status === 204 || (response.status === 404 && method === "DELETE")) return null;
  if (!response.ok) {
    const data = await response.json().catch(() => null);
    const message = response.status === 412 ? "This item changed in Microsoft To Do during synchronization. Sync again to review the conflict."
      : data?.error?.message || `Microsoft Graph returned ${response.status}.`;
    const error = new Error(message);
    error.status = response.status;
    error.code = data?.error?.code;
    throw error;
  }
  return response;
}

export async function graph(path, method = "GET", body, etag, options = {}) {
  let result;
  try {
    const response = await request(path, method, body, etag, 0, false, undefined, options);
    result = response ? await response.json() : null;
    return result;
  } finally {
    if (method !== "GET") { mutationRevision++; reader.invalidate(path, result); }
  }
}

export async function graphBytes(path, options = {}) {
  const response = await request(path, "GET", undefined, undefined, 0, false, { Accept: "*/*" }, options);
  if (!response) throw new Error("Microsoft returned an empty attachment.");
  return boundedBlob(response, options.maxBytes ?? 25 * 1024 * 1024);
}

export async function graphUpload(sessionUrl, chunk, start, total) {
  const url = graphUrl(sessionUrl);
  if (!/\/todo\/lists\/[^/]+\/tasks\/[^/]+\/attachmentSessions\/[^/]+$/i.test(url.pathname) || url.search) {
    throw new Error("Microsoft returned an unexpected attachment upload URL.");
  }
  url.pathname += "/content";
  // Fetch supplies Content-Length for the Blob; browsers forbid setting it directly.
  try {
    const response = await request(url.href, "PUT", chunk, undefined, 0, false, {
      "Content-Type": "application/octet-stream", "Content-Range": `bytes ${start}-${start + chunk.size - 1}/${total}`,
    });
    return { status: response.status, location: response.headers.get("Location"), data: await response.json().catch(() => null) };
  } finally { mutationRevision++; reader.invalidate(sessionUrl); }
}

export async function collection(path, options = {}) {
  const items = [];
  const visited = new Set();
  while (path) {
    const canonical = graphUrl(path).href;
    if (visited.has(canonical) || visited.size >= 10_000) throw new Error("Microsoft returned a repeated or excessive continuation sequence.");
    visited.add(canonical);
    const page = await graph(path, "GET", undefined, undefined, options);
    if (!Array.isArray(page?.value)) throw new Error("Microsoft returned an invalid collection.");
    items.push(...page.value);
    path = page["@odata.nextLink"];
  }
  return items;
}

const reader = createCloudReader({ graph, collection, graphUrl, listPath, taskPath });
export const readCloud = (accountId = "", options) => reader.read(accountId, options);
export const hydrateTask = (listId, taskId) => reader.task(listId, taskId);
export const readList = listId => reader.list(listId);
export const clearCloudCache = () => reader.clear();
