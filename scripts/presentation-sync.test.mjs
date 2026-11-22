import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";

registerHooks({
  resolve(specifier,context,next){const result=next(specifier,context);if(result.url.endsWith("/cloud/client.js"))return{url:"fixture:presentation-client",shortCircuit:true};if(result.url.endsWith("/cloud/auth.js"))return{url:"fixture:presentation-auth",shortCircuit:true};return result;},
  load(url,context,next){if(url==="fixture:presentation-client")return{format:"module",shortCircuit:true,source:"export const segment=encodeURIComponent;export const graph=()=>{throw Error('unexpected request')};export const collection=graph;"};if(url==="fixture:presentation-auth")return{format:"module",shortCircuit:true,source:"export const serviceEnvironment=()=>({id:'global'});"};return next(url,context);}
});
const {createPresentationSync}=await import("../crates/todo-browser/wwwroot/cloud/presentation.js");
const A="11111111-1111-4111-8111-111111111111";
const B="22222222-2222-4222-8222-222222222222";
const G="33333333-3333-4333-8333-333333333333";

function fixture(t){
  const old=globalThis.localStorage,values=new Map();
  const storage={get length(){return values.size;},key:i=>[...values.keys()][i]??null,getItem:key=>values.get(key)??null,setItem:(key,value)=>values.set(key,String(value)),removeItem:key=>values.delete(key)};
  globalThis.localStorage=storage;t.after(()=>{globalThis.localStorage=old;});
  const state={schema_version:1,next_id:5,settings:{theme:"Light",presentation_sync_enabled:true},
    lists:[{id:1,name:"Tasks",color:"#3E69E4",group_id:null}],
    tasks:[{id:2,list_id:1,title:"Task",my_day:null,my_day_excluded:null,position:0,steps:[],attachments:[]}],
    groups:[{id:4,name:"Work",collapsed:false,position:0}],sidebar_order:[4]};
  const h={storage,state,reads:0,writes:0,replacements:0,now:1000,replicas:new Map()};
  storage.setItem("ferroui.todo.graph.v1.account",JSON.stringify({version:1,lists:{1:{remoteId:"default"}},tasks:{2:{listId:"default",remoteId:"task"}}}));
  const save=()=>storage.setItem("ferroui.todo.v1",JSON.stringify(h.state));save();
  h.mutate=change=>{change(h.state);save();};
  h.runtime={exportData:()=>JSON.stringify(h.state),prepareCloudData:json=>json,
    replaceFromCloud:(json,expected)=>{assert.equal(JSON.stringify(h.state),expected);h.state=JSON.parse(json);h.replacements++;save();}};
  let ids=0;
  h.sync=createPresentationSync({storage:()=>storage,uuid:()=>ids++===0?A:G,now:()=>h.now,remote:{
    read:async()=>{h.reads++;return structuredClone([...h.replicas.values()]);},
    write:async(_account,replica)=>{h.writes++;h.replicas.set(replica.device,structuredClone(replica));if(h.afterWrite)await h.afterWrite();}
  }});
  return h;
}

test("no-op device settings sync has no remote writes or document replacements",async t=>{
  const h=fixture(t);await h.sync(h.runtime,"account");const first={writes:h.writes,replacements:h.replacements,reads:h.reads};
  for(let i=0;i<10;i++)await h.sync(h.runtime,"account");
  assert.deepEqual({writes:h.writes,replacements:h.replacements,reads:h.reads},first);
  h.now+=60_001;await h.sync(h.runtime,"account");assert.equal(h.reads,first.reads+1);assert.equal(h.writes,first.writes);assert.equal(h.replacements,first.replacements);
});

test("lost upload response recovers the same replica and preserves a newer local edit",async t=>{
  const h=fixture(t);h.mutate(state=>state.settings.theme="Dark");
  h.afterWrite=async()=>{h.mutate(state=>state.settings.theme="System");throw new Error("response lost");};
  await assert.rejects(h.sync(h.runtime,"account"),/response lost/);
  assert.equal(h.state.settings.theme,"System");assert.equal(h.replicas.size,1);
  h.afterWrite=undefined;await h.sync(h.runtime,"account");
  assert.equal(h.state.settings.theme,"System");assert.equal(h.replicas.size,1);assert.equal(h.state.groups.length,1);
  const key=JSON.stringify(["settings","app","theme"]);assert.equal(h.replicas.get(A).records[key].value,"System");
  assert.equal(JSON.parse(h.storage.getItem("ferroui.todo.presentation.v1.account")).pending,undefined);
});

test("local compare-and-swap protects edits during a successful remote upload",async t=>{
  const h=fixture(t);h.afterWrite=async()=>h.mutate(state=>state.tasks[0].my_day="2026-11-22");
  await assert.rejects(h.sync(h.runtime,"account"),/changed on this device/);
  assert.equal(h.state.tasks[0].my_day,"2026-11-22");assert.equal(h.replacements,0);
  h.afterWrite=undefined;await h.sync(h.runtime,"account");assert.equal(h.state.tasks[0].my_day,"2026-11-22");
});

test("a corrupt or future replica stops before remote writes and local changes",async t=>{
  const h=fixture(t);h.replicas.set(B,{kind:"ferroui.todo.presentation",version:2,device:B,records:{}});
  const before=h.runtime.exportData();await assert.rejects(h.sync(h.runtime,"account"),/unsupported/);
  assert.equal(h.runtime.exportData(),before);assert.equal(h.writes,0);assert.equal(h.replacements,0);
});

test("document account binding prevents cross-account OneDrive publication",async t=>{
  const h=fixture(t);await h.sync(h.runtime,"account");const reads=h.reads,writes=h.writes;
  await assert.rejects(h.sync(h.runtime,"different-account"),/linked to another/);
  assert.equal(h.reads,reads);assert.equal(h.writes,writes);
});

test("manual sync bypasses presentation polling throttle without uploading unchanged data",async t=>{
  const h=fixture(t);await h.sync(h.runtime,"account");await h.sync(h.runtime,"account",{force:true});
  assert.equal(h.reads,2);assert.equal(h.writes,1);
});
