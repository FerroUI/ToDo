import { COMMON, TASK, PUBLIC_STRINGS } from "../vendor/pst-writer/names.js";
import { fileTime, utf16 } from "../vendor/pst-writer/bytes.js";
import { prop } from "../vendor/pst-writer/ltp.js";
import { BODY_TIME_SET, originalNamedProperty, preservedStatus, preservedImportance, preservedPercent, startForExport, sourceTimestamp } from "./pst-source.js";
import { noteFields, richNoteFields } from "../notes.js";
import { encodePstRecurrence } from "./pst-recurrence.js";

// Property IDs below are positions in this PST's named-property map. The LIDs
// describe Outlook properties; they must never be written directly as IDs.
export const taskNames = [
  [TASK,0x8101], [TASK,0x8102], [TASK,0x8104], [TASK,0x8105], [TASK,0x810f], [TASK,0x811c],
  [COMMON,0x8503], [COMMON,0x8502], [COMMON,0x8560], [COMMON,0x8501],
  [COMMON,0x8516], [COMMON,0x8517], [TASK,0x8126], [TASK,0x8116], [TASK,0x8109],
].map(([guid,lid])=>({guid,lid}));
taskNames.push({guid:BODY_TIME_SET,name:"BodyLastModifiedTime"});
taskNames.push({guid:PUBLIC_STRINGS,name:"FerroUI.ToDo.Task"});
const id = (guid,lid) => 0x8002 + taskNames.findIndex(value=>value.guid===guid&&value.lid===lid);
const namedId = (guid,name) => 0x8002 + taskNames.findIndex(value=>value.guid===guid&&value.name===name);
const civil = value => new Date(`${value}T00:00:00.000Z`);
function double(property,value) {
  const data = new Uint8Array(8);new DataView(data.buffer).setFloat64(0,value,true);
  return {id:property,type:0x0005,data};
}
function categories(values) {
  const strings=values.map(utf16), bytes=4+4*strings.length+strings.reduce((sum,value)=>sum+value.length,0);
  const data=new Uint8Array(bytes), view=new DataView(data.buffer);view.setUint32(0,strings.length,true);
  let offset=4+strings.length*4;
  strings.forEach((value,index)=>{view.setUint32(4+index*4,offset,true);data.set(value,offset);offset+=value.length;});
  return {id:0x8000,type:0x101f,data};
}
export function taskProperties(task, body = task.notes || "", html = null) {
  const repeat=encodePstRecurrence(task);
  const properties=[prop.int32(id(TASK,0x8101),preservedStatus(task)),double(id(TASK,0x8102),preservedPercent(task)),
    prop.bool(id(TASK,0x811c),task.completed),prop.int32(0x0017,preservedImportance(task)),
    prop.bool(id(TASK,0x8126),!!repeat),prop.bool(id(COMMON,0x8503),!!task.reminder)];
  if(repeat) properties.push(prop.binary(id(TASK,0x8116),repeat.data),prop.bool(id(TASK,0x8109),repeat.dead));
  if(task.created_on) properties.push(prop.time(0x3007,sourceTimestamp(task,"createdDateTime",0x3007,task.created_on)));
  if(task.updated_on) properties.push(prop.time(0x3008,sourceTimestamp(task,"lastModifiedDateTime",0x3008,task.updated_on)));
  const start=startForExport(task);
  if(start) properties.push(prop.time(id(TASK,0x8104),fileTime(civil(start))),prop.time(id(COMMON,0x8516),fileTime(new Date(`${start}T00:00:00`))));
  if (task.due_date) {
    const due=fileTime(civil(task.due_date));
    properties.push(prop.time(id(TASK,0x8105),due),prop.time(id(COMMON,0x8517),fileTime(new Date(`${task.due_date}T00:00:00`))));
  }
  if (task.completed && task.completed_on) {
    // Completion is midnight in local time expressed as a UTC FILETIME.
    properties.push(prop.time(id(TASK,0x810f),fileTime(new Date(`${task.completed_on}T00:00:00`))));
  }
  if (task.reminder) {
    const reminder=new Date(`${task.reminder.date}T00:00:00`);
    reminder.setMinutes(task.reminder.minutes_after_midnight);
    const time=fileTime(reminder);
    properties.push(prop.time(id(COMMON,0x8502),time),prop.time(id(COMMON,0x8560),time),prop.int32(id(COMMON,0x8501),0));
  }
  if (task.tags?.length) properties.push(categories(task.tags));
  // The Graph/PST mapping specifies this string-named property. Preserve an
  // existing representation instead of inventing a type for unknown producers.
  const bodyTime=originalNamedProperty(task,BODY_TIME_SET,"BodyLastModifiedTime");
  const sourceBody=task.microsoft_data?.body;
  const bodyUnchanged=sourceBody?.contentType==="text" ? body===(sourceBody.content||"") && !html
    : !!html && sourceBody?.contentType==="html" && JSON.stringify(richNoteFields(html))===JSON.stringify(richNoteFields(sourceBody.content));
  if(bodyUnchanged) {
    if(bodyTime?.type==="Time" && /^\d{1,19}$/.test(bodyTime.value)) properties.push(prop.time(namedId(BODY_TIME_SET,"BodyLastModifiedTime"),BigInt(bodyTime.value)));
    else if(["Unicode","String8"].includes(bodyTime?.type) && typeof bodyTime.value==="string") properties.push(prop.string(namedId(BODY_TIME_SET,"BodyLastModifiedTime"),bodyTime.value));
  }
  // Outlook consumes the standard task properties above. These application
  // details remain round-trippable alongside Outlook's native recurrence.
  const note=noteFields(task);
  const metadata={kind:"ferroui.todo.pst",version:task.recurrence||task.start_date?3:html?2:1,notes:note.notes,rendered_body:body,
    ...(html?{notes_html:note.notes_html,rendered_html:html}:{}),
    steps:(task.steps||[]).map(step=>({title:step.title,completed:!!step.completed})),
    my_day:task.my_day||null,my_day_excluded:task.my_day_excluded||null,
    recurrence:task.recurrence||null,
    ...(task.recurrence?{native_recurrence:true,rendered_recurrence:repeat?.rendered||null}:{}),
    ...(task.recurrence_generated?{recurrence_generated:true}:{}),
    ...(task.start_date?{start_date:task.start_date,rendered_start_date:start}:{}),
    linked_resources:task.linked_resources||[]};
  properties.push(prop.string(0x8002+taskNames.length-1,JSON.stringify(metadata)));
  return properties;
}
