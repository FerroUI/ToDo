import { TASK } from "../vendor/pst-writer/names.js";
import { fileTime } from "../vendor/pst-writer/bytes.js";

const statuses = { notStarted: 0, inProgress: 1, completed: 2, waitingOnOthers: 3, deferred: 4 };
const importance = { low: 0, normal: 1, high: 2 };
export const BODY_TIME_SET = "6f305800-7b8b-41af-b73b-84de53a69d3d";

export function originalProperty(task, id) {
  return task.microsoft_data?.pst?.properties?.[id.toString(16).padStart(4,"0")];
}
export function originalNamedProperty(task, guid, name) {
  const original=task.microsoft_data?.pst, identity=typeof name==="number"?name.toString(16).padStart(8,"0"):name;
  const id=Object.keys(original?.namedProperties || {}).find(id=>{
    const entry=original.namedProperties[id];
    return typeof entry?.guid==="string" && entry.guid.toLowerCase()===guid && entry.name===identity;
  });
  return id ? original.properties?.[id] : undefined;
}
function integer(property) {
  if(!["Integer16","Integer32","Integer64"].includes(property?.type)) return undefined;
  const value=Number(property.value);return Number.isSafeInteger(value)?value:undefined;
}
export function sourceStatus(task) {
  const value=statuses[task.microsoft_data?.status] ?? integer(originalNamedProperty(task,TASK,0x8101));
  return [0,1,2,3,4].includes(value)?value:undefined;
}
export function preservedStatus(task) {
  const value=sourceStatus(task);
  return value!==undefined && (value===2)===!!task.completed ? value : task.completed?2:0;
}
export function preservedImportance(task) {
  const value=importance[task.microsoft_data?.importance] ?? integer(originalProperty(task,0x0017));
  return [0,1,2].includes(value) && (value===2)===!!task.important ? value : task.important?2:1;
}
export function preservedPercent(task) {
  if(task.completed) return 1;
  if(sourceStatus(task)===undefined || sourceStatus(task)===2) return 0;
  const property=originalNamedProperty(task,TASK,0x8102),value=property?.value;
  return ["Floating32","Floating64"].includes(property?.type) && Number.isFinite(value) && value>=0 && value<1 ? value:0;
}
export function civilDate(value) {
  const text=typeof value==="string"?value:value?.dateTime;
  if(typeof text!=="string" || !/^\d{4}-\d\d-\d\d(?:T|$)/.test(text)) return null;
  const day=text.slice(0,10),date=new Date(`${day}T00:00:00Z`);
  return Number.isFinite(date.getTime()) && date.toISOString().slice(0,10)===day && day>="1601-01-01" && day<"4501-01-01"?day:null;
}
export function originalTime(property) {
  if(property?.type!=="Time" || typeof property.value!=="string" || !/^\d{1,19}$/.test(property.value)) return null;
  const ticks=BigInt(property.value);
  if(ticks===0n || ticks===0x5ae980e0n || ticks===0x5ae980e0n*600000000n || ticks>2650467743999999999n) return null;
  return ticks;
}
export function sourceStart(task) {
  if(task.microsoft_data?.startDateTime===null) return null;
  const value=civilDate(task.microsoft_data?.startDateTime);
  if(value) return value;
  const ticks=originalTime(originalNamedProperty(task,TASK,0x8104));
  return ticks===null?null:civilDate(new Date(Number(ticks/10000n)-11644473600000).toISOString());
}
export function startForExport(task) {
  const start=task.start_date || sourceStart(task);
  // MS-OXOTASK requires a due date whenever a start date exists, and due>=start.
  return start && task.due_date && start<=task.due_date ? start:null;
}
export function sourceTimestamp(task, field, property, localDate) {
  const raw=originalTime(originalProperty(task,property));
  if(raw!==null) {
    const instant=new Date(Number(raw/10000n)-11644473600000);
    if(civilDate(instant.toISOString())===localDate) return raw;
  }
  const value=task.microsoft_data?.[field];
  if(typeof value==="string" && civilDate(value)===localDate && /(?:Z|[+-]\d\d:\d\d)$/.test(value)) {
    const date=new Date(value);if(Number.isFinite(date.getTime())) return fileTime(date);
  }
  return fileTime(new Date(`${localDate}T00:00:00Z`));
}
