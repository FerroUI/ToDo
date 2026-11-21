import { saveAttachment, deleteAttachment, openAttachment, downloadBlob, chooseFile } from "./attachments.js";
import { requestNotifications, notify, completionSound } from "./notifications.js";
import { microsoftRequest } from "./graph.js";
import { createDocumentStorage } from "./document-storage.js";

const demo = new URLSearchParams(location.search).get("demo") === "1";
const storageKey = demo ? "ferroui.todo.demo.v1" : "ferroui.todo.v1";
const documentStorage = createDocumentStorage(() => localStorage, storageKey);
let runtime;

export function attachRuntime(value) {
  runtime = value;
  // Demo startup builds a sample directly in Rust and does not call load().
  if (demo) {
    try { documentStorage.load(); }
    catch (error) { queueMicrotask(() => reportError(error)); }
  }
}
export function load() { return documentStorage.load(); }
export function save(contents) {
  try { documentStorage.save(contents); }
  // The existing wasm boundary reads JsValue::as_string; preserve the actionable message.
  catch (error) { throw error?.message || String(error); }
}
export function width() { return document.getElementById("out").clientWidth; }
export function systemDark() { return matchMedia("(prefers-color-scheme: dark)").matches; }
export function preserveRecovery(contents) {
  try { localStorage.setItem(`${storageKey}.recovery.${Date.now()}`, contents); }
  catch { downloadBlob(new Blob([contents], { type: "application/json" }), "todo-recovery.json"); }
}
export function today() {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
}
export function reportError(error) { runtime?.hostError(error?.message || String(error)); }

export function request(json) {
  try {
    const action = JSON.parse(json);
    const pending = handle(action);
    pending?.catch(reportError);
  } catch (error) { reportError(error); }
}

async function handle(action) {
  switch (action.kind) {
    case "connectMicrosoft":
    case "disconnectMicrosoft":
    case "syncMicrosoft":
      await microsoftRequest(action, today()); break;
    case "export":
      downloadBlob(new Blob([action.contents], { type: "application/json" }), action.filename);
      break;
    case "import": {
      const [file] = await chooseFile("application/json,.json");
      if (file) {
        if (file.size > 25 * 1024 * 1024) throw new Error("The backup must be 25 MB or smaller.");
        runtime.importData(await file.text());
      }
      break;
    }
    case "attach": {
      const files = await chooseFile("", true);
      for (const file of files) {
        const key = await saveAttachment(file);
        try { runtime.attachFile(String(action.task_id), file.name, file.type, file.size, key); }
        catch (error) { await deleteAttachment(key); throw error; }
      }
      break;
    }
    case "openAttachment": await openAttachment(action.key); break;
    case "deleteAttachment": await deleteAttachment(action.key); break;
    case "requestNotifications": await requestNotifications(); break;
    case "notify": await notify(action.title, action.body); break;
    case "completionSound": await completionSound(); break;
    case "setTheme": {
      const theme = String(action.theme).toLowerCase();
      document.body.dataset.theme = theme;
      document.querySelector('meta[name="theme-color"]').content = theme === "dark" ? "#202020" : "#f3f3f3";
      break;
    }
    case "openUrl": {
      const url = new URL(action.url, location.href);
      if (!["https:", "http:", "mailto:"].includes(url.protocol)) throw new Error("This link type is not supported.");
      window.open(url.href, "_blank", "noopener,noreferrer");
      break;
    }
    case "print": {
      const print = window.open("", "_blank");
      if (!print) throw new Error("Allow popups to print this list.");
      print.document.open();
      const title = action.title.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
      print.document.write(`<!doctype html><html><head><meta charset="utf-8"><title>${title}</title><style>body{font:15px "Segoe UI",system-ui,sans-serif;max-width:760px;margin:48px auto;color:#252423}h1{font-size:30px}article{padding:12px 0;border-bottom:1px solid #e5e5e5;break-inside:avoid}h3{font-size:16px;font-weight:500;margin:0 0 8px}p{font-size:13px;color:#605e5c;margin:6px 0}.notes{white-space:pre-wrap}@media print{body{margin:0}article{break-inside:avoid}}</style></head><body><h1>${title}</h1>${action.html}</body></html>`);
      print.document.close();
      print.document.title = action.title;
      print.addEventListener("afterprint", () => print.close(), { once: true });
      setTimeout(() => print.print(), 150);
      break;
    }
    default: throw new Error(`Unsupported action: ${action.kind}`);
  }
}
