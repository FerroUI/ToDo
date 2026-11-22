import { chooseFile, storeAttachment, deleteAttachment } from "../attachments.js";
import { restorePortable } from "./archive.js";
import { documentGuard } from "./document.js";
import { MAX_ARCHIVE_BYTES } from "./codec.js";
import { remoteFields } from "../cloud/codec.js";
import { PST_RESOURCE_ERROR } from "./pst-errors.js";

export function parsePstInWorker(bytes, current, today, {
  workerFactory = () => new Worker(new URL("./pst-worker.js", import.meta.url), { type: "module" }),
  signal, timeout = 60_000,
} = {}) {
  return new Promise((resolve, reject) => {
    if (signal?.aborted) return reject(new DOMException("PST import cancelled.", "AbortError"));
    let worker, timer, settled = false;
    const finish = (error, value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      signal?.removeEventListener("abort", cancel);
      worker?.terminate();
      error ? reject(error) : resolve(value);
    };
    const cancel = () => finish(new DOMException("PST import cancelled.", "AbortError"));
    try {
      worker = workerFactory();
      worker.onmessage = ({ data }) => {
        if (typeof data?.error === "string") finish(new Error(data.error));
        else if (typeof data?.result === "string") finish(null, data.result);
        else finish(new Error("The PST parser returned an invalid result."));
      };
      worker.onerror = () => finish(new Error(PST_RESOURCE_ERROR));
      worker.onmessageerror = () => finish(new Error("The PST parser result could not be transferred."));
      signal?.addEventListener("abort", cancel, { once: true });
      timer = setTimeout(() => finish(new Error("PST parsing took too long. Export a smaller task-only PST in Outlook and try again.")), timeout);
      worker.postMessage({ bytes, current, today }, [bytes]);
    } catch (error) { finish(error); }
  });
}

// Outlook stores reminder instants in UTC, while the Rust UI uses local civil
// dates/times. Date resolves the browser/native host's timezone and DST for the
// reminder's date; using today's fixed UTC offset would shift future reminders.
export function normalizePstImport(report) {
  if (!report || !report.archive || !Array.isArray(report.archive.document?.tasks)
    || !Array.isArray(report.warnings) || report.warnings.some(value => typeof value !== "string")
    || !Number.isSafeInteger(report.skipped_messages) || report.skipped_messages < 0
    || !Array.isArray(report.utc_reminders)) throw new Error("The PST parser returned invalid task data.");
  const tasks = new Map(report.archive.document.tasks.map(task => [task.id, task])), seen = new Set();
  for (const task of tasks.values()) {
    if (task.notes === "" && task.microsoft_data?.body?.contentType === "html"
      && !task.microsoft_data?.pst?.properties?.["1000"]) task.notes=remoteFields(task.microsoft_data).notes;
  }
  for (const reminder of report.utc_reminders) {
    const task = tasks.get(reminder?.task_id), value = reminder?.date_time;
    if (!task || seen.has(task.id) || typeof value !== "string" || !/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$/.test(value)) throw new Error("The PST parser returned an invalid reminder.");
    seen.add(task.id);
    const date = new Date(value);
    if (!Number.isFinite(date.getTime())) throw new Error("A PST reminder is outside the supported date range.");
    const day = `${String(date.getFullYear()).padStart(4,"0")}-${String(date.getMonth()+1).padStart(2,"0")}-${String(date.getDate()).padStart(2,"0")}`;
    task.reminder = { date: day, minutes_after_midnight: date.getHours() * 60 + date.getMinutes(), notified: false };
  }
  if (!Array.isArray(report.utc_dates || [])) throw new Error("The PST parser returned invalid dates.");
  for (const value of report.utc_dates || []) {
    const task = tasks.get(value?.task_id);
    if (!task || value.field !== "completed_on" || typeof value.date_time !== "string"
      || !/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$/.test(value.date_time)) throw new Error("The PST parser returned an invalid completion date.");
    const date = new Date(value.date_time);
    if (!Number.isFinite(date.getTime())) throw new Error("A PST completion date is outside the supported date range.");
    task.completed_on = `${String(date.getFullYear()).padStart(4,"0")}-${String(date.getMonth()+1).padStart(2,"0")}-${String(date.getDate()).padStart(2,"0")}`;
  }
  return report;
}

export async function chooseAndRestorePst(runtime, {
  choose = chooseFile, storeFile = storeAttachment, deleteFile = deleteAttachment,
  parse = parsePstInWorker, signal,
} = {}) {
  const [file] = await choose(".pst,application/vnd.ms-outlook");
  if (!file) return null;
  if (!(file instanceof Blob) || file.size > MAX_ARCHIVE_BYTES) throw new Error("The Outlook PST must be 256 MiB or smaller.");
  const expected = runtime.exportData(), guard = documentGuard(runtime, expected), now = new Date();
  const today = `${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,"0")}-${String(now.getDate()).padStart(2,"0")}`;
  const bytes = await file.arrayBuffer();
  guard();
  const report = normalizePstImport(JSON.parse(await parse(bytes, expected, today, { signal })));
  guard();
  const restored = await restorePortable(runtime, JSON.stringify(report.archive), { storeFile, deleteFile });
  return { ...restored, warnings: report.warnings, skipped_messages: report.skipped_messages };
}
