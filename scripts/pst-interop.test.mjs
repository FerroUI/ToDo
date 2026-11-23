import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fixturePst, fidelityPst } from "./tests/pst-fixture.mjs";
import { richNotesPst } from "./tests/rich-notes-pst-fixture.mjs";
import { chooseAndRestorePst, normalizePstImport, parsePstInWorker } from "../crates/todo-browser/wwwroot/backup/pst.js";
import { exportPst } from "../crates/todo-browser/wwwroot/backup/pst-export.js";
import { PST_RESOURCE_ERROR, pstParserError } from "../crates/todo-browser/wwwroot/backup/pst-errors.js";
import { task } from "./tests/graph-fixture.mjs";
import { taskNames, taskProperties } from "../crates/todo-browser/wwwroot/backup/pst-properties.js";
import { sourceStart, startForExport, preservedStatus, preservedImportance } from "../crates/todo-browser/wwwroot/backup/pst-source.js";
import { TASK, COMMON } from "../crates/todo-browser/wwwroot/vendor/pst-writer/names.js";

function runtime() {
  let document={schema_version:1,tasks:[task(3,"Current task")],lists:[{id:1,name:"Tasks"}],groups:[],next_id:10,settings:{auto_sync:true}};
  const calls=[];
  return {calls,exportData:()=>JSON.stringify(document),validateImportData:json=>json,importData:json=>{document=JSON.parse(json);calls.push("import");},
    get state(){return document;},mutate(){document.tasks[0].title="Changed during parsing";}};
}
function report() {
  return {archive:{kind:"ferroui.todo.backup",version:1,document:{schema_version:1,tasks:[task(2,"Imported task")],lists:[{id:1,name:"Tasks"}],groups:[],next_id:3,settings:{}},files:[]},
    warnings:["Skipped 1 non-task item."],skipped_messages:1,utc_reminders:[],utc_dates:[]};
}

test("PST fixture matches the independently parsed Microsoft Rust-reader fixture",async()=>{
  const zone=process.env.TZ;process.env.TZ="UTC";
  try {
    const actual=new Uint8Array(await fixturePst().arrayBuffer()),expected=await readFile(new URL("../crates/todo-interop/tests/fixtures/tasks.pst",import.meta.url));
    assert.deepEqual(actual,new Uint8Array(expected));
    const fidelity=new Uint8Array(await fidelityPst().arrayBuffer());
    assert.deepEqual(fidelity,new Uint8Array(await readFile(new URL("../crates/todo-interop/tests/fixtures/fidelity.pst",import.meta.url))));
    assert.deepEqual(new Uint8Array(await richNotesPst().arrayBuffer()),new Uint8Array(await readFile(new URL("../crates/todo-interop/tests/fixtures/rich-notes.pst",import.meta.url))));
  }finally{if(zone===undefined)delete process.env.TZ;else process.env.TZ=zone;}
});

test("PST exports source status/importance/start and respects local edits and UTC companion dates",()=>{
  const zone=process.env.TZ;process.env.TZ="Europe/Warsaw";
  try {
    const value={...task(3,"Original"),due_date:"2027-07-21",microsoft_data:{status:"waitingOnOthers",importance:"low",
      startDateTime:{dateTime:"2027-07-01T00:00:00",timeZone:"UTC"}}};
    assert.equal(preservedStatus(value),3);assert.equal(preservedImportance(value),0);
    const properties=taskProperties(value),named=(guid,lid)=>properties.find(property=>property.id===0x8002+taskNames.findIndex(entry=>entry.guid===guid && entry.lid===lid));
    const ticks=property=>new DataView(property.data.buffer,property.data.byteOffset,8).getBigUint64(0,true);
    assert.equal(ticks(named(TASK,0x8104))-ticks(named(COMMON,0x8516)),2n*60n*60n*10000000n);
    assert.equal(ticks(named(TASK,0x8105))-ticks(named(COMMON,0x8517)),2n*60n*60n*10000000n);
    assert.equal(preservedStatus({...value,completed:true}),2);
    assert.equal(preservedImportance({...value,important:true}),2);
    assert.equal(startForExport({...value,due_date:null}),null);
    assert.equal(startForExport({...value,due_date:"2027-06-30"}),null);
    assert.equal(sourceStart(value),"2027-07-01");
    const original={...value,microsoft_data:{pst:{properties:{"0017":{type:"Integer32",value:0},beef:{type:"Integer32",value:4}},
      namedProperties:{beef:{guid:TASK,name:"00008101"}}}}};
    assert.equal(preservedStatus(original),4);assert.equal(preservedImportance(original),0);
  }finally{if(zone===undefined)delete process.env.TZ;else process.env.TZ=zone;}
});

test("PST restore parses before storing and commits independent task data once",async()=>{
  const host=runtime();let bytes;
  const result=await chooseAndRestorePst(host,{choose:async()=>[new File(["!BDN fixture"],"tasks.pst")],parse:async value=>{bytes=value;return JSON.stringify(report());}});
  assert.ok(bytes instanceof ArrayBuffer);
  assert.equal(result.tasks,1);assert.deepEqual(result.warnings,["Skipped 1 non-task item."]);
  assert.deepEqual(host.calls,["import"]);assert.equal(host.state.settings.auto_sync,false);
});

test("PST concurrent edits and parser failure cannot replace current tasks",async()=>{
  const host=runtime(),choose=async()=>[new File(["!BDN"],"tasks.pst")];
  await assert.rejects(chooseAndRestorePst(host,{choose,parse:async()=>{host.mutate();return JSON.stringify(report());}}),/Tasks changed/);
  assert.equal(host.state.tasks[0].title,"Changed during parsing");assert.deepEqual(host.calls,[]);
  await assert.rejects(chooseAndRestorePst(host,{choose,parse:async()=>{throw new Error("Malformed PST");}}),/Malformed PST/);
  assert.deepEqual(host.calls,[]);
});

test("PST UTC reminders and completion dates use the date's daylight-saving offset",()=>{
  const zone=process.env.TZ;process.env.TZ="Europe/Warsaw";
  try {
    const value=report();value.utc_reminders=[{task_id:2,date_time:"2027-07-01T08:15:00Z"}];
    value.utc_dates=[{task_id:2,field:"completed_on",date_time:"2026-11-19T23:00:00Z"}];
    const task=normalizePstImport(value).archive.document.tasks[0];
    assert.deepEqual(task.reminder,{date:"2027-07-01",minutes_after_midnight:615,notified:false});
    assert.equal(task.completed_on,"2026-11-20");
    assert.throws(()=>normalizePstImport({...report(),utc_reminders:[{task_id:999,date_time:"2027-07-01T08:15:00Z"}]}),/invalid reminder/);
  }finally{if(zone===undefined)delete process.env.TZ;else process.env.TZ=zone;}
});

test("PST worker cancellation, timeout, and completion always release worker resources",async()=>{
  const make=()=>({terminated:0,postMessage(){},terminate(){this.terminated++;}});
  const worker=make(),controller=new AbortController();
  const pending=parsePstInWorker(new ArrayBuffer(4),"{}","2026-11-22",{workerFactory:()=>worker,signal:controller.signal});
  controller.abort();await assert.rejects(pending,{name:"AbortError"});assert.equal(worker.terminated,1);
  const timed=make();await assert.rejects(parsePstInWorker(new ArrayBuffer(4),"{}","2026-11-22",{workerFactory:()=>timed,timeout:1}),/too long/);assert.equal(timed.terminated,1);
  const complete=make(),done=parsePstInWorker(new ArrayBuffer(4),"{}","2026-11-22",{workerFactory:()=>complete});
  complete.onmessage({data:{result:"{}"}});assert.equal(await done,"{}");assert.equal(complete.terminated,1);
  const failed=make(),failure=parsePstInWorker(new ArrayBuffer(4),"{}","2026-11-22",{workerFactory:()=>failed});
  failed.onmessage({data:{error:pstParserError(new WebAssembly.RuntimeError("unreachable"))}});
  await assert.rejects(failure,{message:PST_RESOURCE_ERROR});assert.equal(failed.terminated,1);
  assert.equal(pstParserError(new RangeError("Array buffer allocation failed")),PST_RESOURCE_ERROR);
  assert.equal(pstParserError(new Error("Invalid PST named properties")),"Invalid PST named properties");
});

test("PST export preserves binary files and fails before download when a file is unavailable",async()=>{
  const host=runtime();host.state.tasks[0].attachments=[{id:4,name:"sample.bin",mime_type:"application/octet-stream",size:4,storage_key:"local:file"}];
  const result=await exportPst(host,{resolveFile:async()=>new Blob([new Uint8Array([0,1,254,255])])});
  const bytes=new Uint8Array(await result.blob.arrayBuffer());
  assert.equal(new TextDecoder().decode(bytes.slice(0,4)),"!BDN");assert.equal(result.files,1);assert.deepEqual(host.calls,[]);
  await assert.rejects(exportPst(host,{resolveFile:async()=>null}),/unavailable/);
  await assert.rejects(exportPst(host,{resolveFile:async()=>{host.mutate();return new Blob(["file"]);}}),/Tasks changed/);
});
