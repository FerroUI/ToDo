import { accessToken } from "./auth.js";

const ROOT = "https://graph.microsoft.com/v1.0/";
export const segment = value => encodeURIComponent(value);
export const listPath = id => `me/todo/lists/${segment(id)}`;
export const taskPath = (listId, id) => `${listPath(listId)}/tasks/${segment(id)}`;

export async function graph(path, method = "GET", body, etag, attempt = 0) {
  const url = new URL(path, ROOT);
  if (url.origin !== "https://graph.microsoft.com" || !url.pathname.startsWith("/v1.0/")) {
    throw new Error("Microsoft returned an unexpected continuation URL.");
  }
  const headers = { Authorization: `Bearer ${await accessToken(attempt > 0)}`, Accept: "application/json" };
  if (body !== undefined) headers["Content-Type"] = "application/json";
  if (etag) headers["If-Match"] = etag;
  const response = await fetch(url, {
    method, headers, body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(30_000), cache: "no-store", credentials: "omit", redirect: "error",
  });
  if ((response.status === 429 || response.status === 503 || response.status === 401) && attempt < 2) {
    const seconds = Math.min(20, Math.max(1, Number(response.headers.get("Retry-After")) || 2 ** attempt));
    await new Promise(resolve => setTimeout(resolve, seconds * 1000));
    return graph(path, method, body, etag, attempt + 1);
  }
  if (response.status === 204 || (response.status === 404 && method === "DELETE")) return null;
  const data = await response.json().catch(() => null);
  if (!response.ok) {
    const message = response.status === 412 ? "This item changed in Microsoft To Do during synchronization. Sync again to review the conflict."
      : data?.error?.message || `Microsoft Graph returned ${response.status}.`;
    const error = new Error(message);
    error.status = response.status;
    throw error;
  }
  return data;
}

export async function collection(path) {
  const items = [];
  const visited = new Set();
  while (path) {
    if (visited.has(path)) throw new Error("Microsoft returned a repeated continuation URL.");
    visited.add(path);
    const page = await graph(path);
    items.push(...(page.value || []));
    path = page["@odata.nextLink"];
  }
  return items;
}

export async function readCloud() {
  const lists = await collection("me/todo/lists");
  for (const list of lists) {
    list.tasks = await collection(`${listPath(list.id)}/tasks?$expand=checklistItems,linkedResources`);
    for (const task of list.tasks) {
      const path = taskPath(list.id, task.id);
      if (!Array.isArray(task.checklistItems) || task["checklistItems@odata.nextLink"]) task.checklistItems = await collection(`${path}/checklistItems`);
      if (!Array.isArray(task.linkedResources) || task["linkedResources@odata.nextLink"]) task.linkedResources = await collection(`${path}/linkedResources`);
    }
  }
  return lists;
}
