import { downloadBlob, readAttachment } from "../attachments.js";
import { documentFrom, documentGuard, references } from "./document.js";
import { MAX_ARCHIVE_BYTES } from "./codec.js";
import { MemorySink, PstStore } from "../vendor/pst-writer/index.js";
import { taskNames, taskProperties } from "./pst-properties.js";
import { encodePstRecurrence } from "./pst-recurrence.js";
import { sourceStart, startForExport } from "./pst-source.js";
import { noteFields, noteBody, escapeNote } from "../notes.js";

function paths(document) {
  const groups=new Map((document.groups||[]).map(group=>[group.id,group.name]));
  const seen=new Set(), result=new Map();
  const segment=name=>String(name).trim().replaceAll("/","／").replaceAll("\\","＼") || "Tasks";
  for (const list of document.lists) {
    const group=groups.get(list.group_id), base=group?`${segment(group)}/${segment(list.name)}`:segment(list.name);
    let path=base, suffix=2;
    while(seen.has(path.toLocaleLowerCase("en-US"))) path=`${base} (${suffix++})`;
    seen.add(path.toLocaleLowerCase("en-US"));result.set(list.id,path);
  }
  return result;
}

export async function exportPst(runtime,{resolveFile=reference=>readAttachment(reference.storage_key)}={}) {
  const expected=runtime.exportData(), guard=documentGuard(runtime,expected), document=documentFrom(runtime,expected);
  const sources=references(document), files=new Map();
  for(const [key,reference] of sources) {
    guard();const file=await resolveFile(reference);guard();
    if(!(file instanceof Blob)||file.size!==reference.size) throw new Error(`“${reference.name}” is unavailable or changed. Connect its Microsoft account and refresh before exporting.`);
    files.set(key,new Uint8Array(await file.arrayBuffer()));guard();
  }
  const sink=new MemorySink(), store=new PstStore(sink,"To Do tasks",taskNames), folders=new Map();
  for(const [id,path] of paths(document)) folders.set(id,store.folder(path,"IPF.Task"));
  for(const task of document.tasks) {
    guard();
    const steps=(task.steps||[]).map(step=>`${step.completed?"[x]":"[ ]"} ${step.title}`);
    const body=steps.length?`${task.notes||""}${task.notes?"\n\n":""}Steps:\n${steps.join("\n")}`:task.notes||"";
    const rich=noteFields(task).notes_html;
    const html=rich?`${noteBody(task).content}${steps.length?`<p style="white-space:pre-wrap"></p><p style="white-space:pre-wrap">Steps:</p>${steps.map(step=>`<p style="white-space:pre-wrap">${escapeNote(step)}</p>`).join("")}`:""}`:null;
    const received=new Date(`${task.created_on}T00:00:00Z`), sent=new Date(`${task.updated_on}T00:00:00Z`);
    store.addMessage(folders.get(task.list_id),{messageClass:"IPM.Task",subject:task.title,body,html,
      senderName:"",senderEmail:"",displayTo:"",received,sent,recipients:[],
      attachments:(task.attachments||[]).map(file=>({name:file.name,mime:file.mime_type,data:files.get(file.storage_key)})),
      extraProperties:taskProperties(task,body,html)});
    if(store.size>MAX_ARCHIVE_BYTES-4*1024*1024) throw new Error("The Outlook PST exceeds the 256 MiB export limit.");
    // Yield between records so a large task list does not monopolize the UI.
    if(store.messages%32===0) await new Promise(resolve=>setTimeout(resolve,0));
  }
  guard();store.finish();
  const blob=sink.blob();
  if(blob.size>MAX_ARCHIVE_BYTES) throw new Error("The Outlook PST exceeds the 256 MiB export limit.");
  const warnings=[];
  if(document.tasks.some(task=>task.recurrence && !encodePstRecurrence(task))) warnings.push("A repeat schedule exceeds Outlook's native recurrence formats. Its full rule is preserved for FerroUI restore; that Outlook task is a standalone occurrence.");
  if(document.tasks.some(task=>task.steps?.length)) warnings.push("Steps appear in Outlook task notes and retain their structure for FerroUI restore.");
  if(document.tasks.some(task=>(task.start_date || sourceStart(task)) && !startForExport(task))) warnings.push("A preserved start date could not be written because Outlook requires a due date on or after it. The local task dates were kept.");
  return {blob,warnings,tasks:document.tasks.length,files:files.size};
}

export async function downloadPst(runtime,{download=downloadBlob,...options}={}) {
  const result=await exportPst(runtime,options);download(result.blob,"todo-outlook-tasks.pst");return result;
}
