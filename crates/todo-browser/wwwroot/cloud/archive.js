import { readCloud as readAccount } from "./client.js";
import { cloudAttachmentKey, materializeCloudAttachment } from "./attachment-content.js";
import { archiveReferences } from "./archive-model.js";
import { microsoftToPortable } from "./archive-import.js";
import { checkTextSize, digest, encodeFile, MAX_ARCHIVE_BYTES, MAX_DOCUMENT_BYTES } from "../backup/codec.js";
import { documentGuard } from "../backup/document.js";
import { restorePortable } from "../backup/archive.js";

// This is the application's versioned Microsoft Graph archive, not Microsoft's
// PST export or its separately announced retirement-export format.
export async function exportMicrosoftArchive(accountId, { readCloud = readAccount, readFile = materializeCloudAttachment } = {}) {
  if (typeof accountId !== "string" || !accountId) throw new Error("Connect a Microsoft account before exporting its tasks.");
  const lists = structuredClone(await readCloud(accountId, { fresh: true }));
  checkTextSize(JSON.stringify(lists), MAX_DOCUMENT_BYTES, "Microsoft task metadata");
  const references = archiveReferences(lists), files = [];
  for (const reference of references.values()) {
    const source = reference.source;
    const key = cloudAttachmentKey(accountId, source.listId, source.taskId, reference.file);
    const blob = await readFile(key, accountId);
    if (!(blob instanceof Blob) || blob.size !== reference.size) throw new Error(`Microsoft file “${reference.name}” is missing or incomplete. Refresh the account and export again.`);
    const bytes = new Uint8Array(await blob.arrayBuffer());
    files.push({ key: `backup:${files.length + 1}`, source, name: reference.name, type: reference.type, size: reference.size,
      base64: encodeFile(bytes), sha256: await digest(bytes) });
  }
  const archive = new Blob([JSON.stringify({ kind: "ferroui.microsoft-todo", version: 1, exportedAt: new Date().toISOString(), lists, files })], { type: "application/json" });
  if (archive.size > MAX_ARCHIVE_BYTES) throw new Error("The Microsoft account archive exceeds 256 MB.");
  return archive;
}

export async function restoreMicrosoftArchive(runtime, text, storageOptions) {
  const expected = runtime.exportData(), guard = documentGuard(runtime, expected);
  checkTextSize(text, MAX_ARCHIVE_BYTES, "The Microsoft archive");
  let archive;
  try { archive = JSON.parse(text); } catch { throw new Error("The Microsoft archive is not valid JSON."); }
  const portable = microsoftToPortable(runtime, archive);
  guard();
  return restorePortable(runtime, JSON.stringify(portable), storageOptions);
}
