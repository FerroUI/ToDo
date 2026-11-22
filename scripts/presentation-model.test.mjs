import test from "node:test";
import assert from "node:assert/strict";
import { PRESENTATION_KIND, fieldKey, mergeRecords, validateReplica } from "../crates/todo-browser/wwwroot/cloud/presentation-format.js";
import { assignGroupIdentities, projectPresentation, recordLocalChanges, applyPresentation } from "../crates/todo-browser/wwwroot/cloud/presentation-model.js";

const A = "11111111-1111-4111-8111-111111111111", B = "22222222-2222-4222-8222-222222222222";
const G = "33333333-3333-4333-8333-333333333333";
const graph = { lists: {1:{remoteId:"default"},2:{remoteId:"projects"}},tasks:{3:{listId:"projects",remoteId:"task"}} };
const state = () => ({ schema_version:1,next_id:5,
  settings:{theme:"Dark",auto_sync:true,planner_enabled:true,presentation_sync_enabled:true,microsoft_client_id:"private-config",smart_colors:{MyDay:"#112233"}},
  lists:[{id:1,name:"Tasks",color:"#3E69E4",group_id:null},{id:2,name:"Work",color:"#8844AA",group_id:4,sort:"Manual",position:2}],
  tasks:[{id:3,list_id:2,title:"Source title",my_day:"2026-11-22",my_day_excluded:null,position:0,notes:"Never presentation data"}],
  groups:[{id:4,name:"Projects",collapsed:true,position:0}],sidebar_order:[4] });
const record = (value,counter=1,device=A) => ({clock:[counter,device],deleted:false,value});

test("presentation projection contains only allowed preferences and stable service identities", () => {
  const projection=projectPresentation(state(),graph,{4:G});
  assert.equal(projection[fieldKey("task",JSON.stringify(["todo","projects","task"]),"my_day")],"2026-11-22");
  assert.equal(projection[fieldKey("list","projects","group")],G);
  assert.equal(projection[fieldKey("group",G,"name")],"Projects");
  const serialized=JSON.stringify(projection);
  for(const secret of ["private-config","Never presentation data","Source title","auto_sync","planner_enabled","presentation_sync_enabled"]) assert(!serialized.includes(secret));
});

test("a second device adopts groups, My Day, themes and sidebar order using its own IDs", () => {
  const source=state(), local=projectPresentation(source,graph,{4:G});
  const {records}=recordLocalChanges(local,{}, {},A);
  const target=state(); target.groups=[];target.lists[1].id=20;target.lists[1].group_id=null;
  target.tasks[0].id=30;target.tasks[0].list_id=20;target.tasks[0].my_day=null;target.settings.theme="Light";target.sidebar_order=[20];target.next_id=40;
  const targetGraph={lists:{1:{remoteId:"default"},20:{remoteId:"projects"}},tasks:{30:{listId:"projects",remoteId:"task"}}};
  const updated=recordLocalChanges(projectPresentation(target,targetGraph,{}),{},records,B);
  assert.equal(updated.changed,false,"first enable does not overwrite remote settings with fresh-device defaults");
  const result=applyPresentation(target,targetGraph,{},updated.records);
  assert.equal(result.draft.tasks[0].id,30);assert.equal(result.draft.tasks[0].my_day,"2026-11-22");
  assert.equal(result.draft.settings.theme,"Dark");assert.equal(result.draft.groups[0].id,40);
  assert.equal(result.draft.groups[0].name,"Projects");assert.equal(result.draft.lists[1].group_id,40);
  assert.deepEqual(result.draft.sidebar_order,[40]);
});

test("concurrent independent edits merge and same-field revisions converge regardless of order", () => {
  const title=fieldKey("group",G,"name"), collapsed=fieldKey("group",G,"collapsed");
  const left={[title]:record("Team",2,A),[collapsed]:record(false,1,A)};
  const right={[title]:record("Projects",1,A),[collapsed]:record(true,2,B)};
  assert.deepEqual(mergeRecords(left,right),mergeRecords(right,left));
  assert.equal(mergeRecords(left,right)[title].value,"Team");assert.equal(mergeRecords(left,right)[collapsed].value,true);
  const conflict={[title]:record("Research",2,B)};
  assert.deepEqual(mergeRecords(left,right,conflict),mergeRecords(conflict,left,right));
  assert.equal(mergeRecords(left,conflict)[title].value,"Research");
  assert.deepEqual(mergeRecords(left,left),left);
});

test("group deletion tombstones remove memberships without deleting tasks or lists", () => {
  const original=state(), projection=projectPresentation(original,graph,{4:G});
  const initial=recordLocalChanges(projection,{}, {},A).records;
  const removed=state();removed.groups=[];removed.lists[1].group_id=null;removed.sidebar_order=[2];
  const updated=recordLocalChanges(projectPresentation(removed,graph,{4:G}),projection,initial,A);
  const result=applyPresentation(original,graph,{4:G},updated.records);
  assert.equal(result.draft.groups.length,0);assert.equal(result.draft.lists[1].group_id,null);
  assert.equal(result.draft.tasks.length,1);assert.equal(result.draft.lists.length,2);
  assert.deepEqual(result.draft.sidebar_order,[2]);
});

test("equal revision with different contents is rejected instead of resolving nondeterministically", () => {
  const key=fieldKey("settings","app","theme");
  assert.throws(()=>mergeRecords({[key]:record("Light")},{[key]:record("Dark")}),/same revision/);
});

test("untrusted cloud settings cannot change account, sync flags, task titles, or arbitrary values", () => {
  const envelope=records=>({kind:PRESENTATION_KIND,version:1,device:A,records});
  for(const key of [fieldKey("settings","app","microsoft_client_id"),fieldKey("settings","app","auto_sync"),fieldKey("task","any","title")]) {
    assert.throws(()=>validateReplica(envelope({[key]:record("injected")})),/unsupported/);
  }
  assert.throws(()=>validateReplica(envelope({[fieldKey("list","projects","color")]:record("url(https://example.test)")})),/invalid/);
  assert.throws(()=>validateReplica(envelope({[fieldKey("task","any","my_day")]:record("2026-02-31")})),/invalid/);
  assert.throws(()=>validateReplica({...envelope({}),version:2}),/unsupported/);
});

test("Planner-only local importance cannot overwrite ordinary To Do importance", () => {
  const input=state();input.tasks[0].important=false;
  const records=recordLocalChanges(projectPresentation(input,graph,{4:G}),{}, {},A).records;
  records[fieldKey("task",JSON.stringify(["todo","projects","task"]),"important")]=record(true,2,B);
  assert.equal(applyPresentation(input,graph,{4:G},records).draft.tasks[0].important,false);
});

test("group identities persist and cannot alias two local groups", () => {
  assert.deepEqual(assignGroupIdentities(state(),{4:G},()=>{throw new Error("must reuse");}),{4:G});
  assert.throws(()=>assignGroupIdentities(state(),{4:G,6:G}),/mappings/);
});
