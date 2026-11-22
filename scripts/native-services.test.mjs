import test from "node:test";
import assert from "node:assert/strict";
import { writeFileSync, readFileSync, readdirSync, mkdirSync } from "node:fs";
import { join } from "node:path";
import { callbackResult, nativeRedirect, receiveAuthorization } from "../crates/todo-desktop/native/loopback.mjs";
import { nativeHost } from "./tests/native-host-fixture.mjs";
import { task } from "./tests/graph-fixture.mjs";
import { NativeDOMParser } from "../crates/todo-desktop/native/html.mjs";
import { cacheScope } from "../crates/todo-desktop/native/cache-format.mjs";
import { keyName } from "../crates/todo-desktop/native/storage.mjs";

const document=()=>({schema_version:1,tasks:[task(2)],lists:[{id:1,name:"Tasks"}],groups:[],sidebar_order:[],next_id:10,settings:{auto_sync:true,planner_enabled:false}});

test("native Graph HTML conversion uses an inert HTML5 tree with entities and block boundaries",()=>{
  const parsed=new NativeDOMParser().parseFromString('<p>Hello &amp; world<br>next</p><div>One<div>Two</div></div><img src="https://invalid.invalid/never-fetch">',"text/html");
  for(const element of parsed.querySelectorAll("br")) element.replaceWith("\n");
  for(const element of parsed.querySelectorAll("p,div,li")) element.append("\n");
  assert.equal(parsed.body.textContent.trimEnd(),"Hello & world\nnext\nOneTwo");
});

test("native OAuth accepts only loopback redirects and exact state/host/path callbacks",()=>{
  for(const address of ["https://localhost/","http://example.com/","http://user@localhost/","http://localhost/?token=leak","http://localhost/#fragment"]) assert.throws(()=>nativeRedirect(address));
  const redirect=nativeRedirect("http://localhost:49152/"), request={method:"GET",headers:{host:redirect.host},url:"/?code=secret&state=expected"};
  assert.deepEqual(callbackResult(request,redirect,"expected"),{code:"secret"});
  for(const changed of [{method:"POST"},{headers:{host:"attacker.test"}},{url:"/other?code=secret&state=expected"},{url:"/?code=secret&state=wrong"},{url:"/?code=secret&state=expected&state=expected"},{url:"/?code=a&code=b&state=expected"}]) assert.equal(callbackResult({...request,...changed},redirect,"expected"),null);
  assert.match(callbackResult({...request,url:"/?error=access_denied&state=expected"},redirect,"expected").error,/cancelled/);
});

test("native loopback receives one valid authorization after rejecting an unrelated request",async()=>{
  const result=await receiveAuthorization({redirectUri:"http://127.0.0.1/",timeout:5000,
    authorize:({redirectUri,state})=>`https://identity.invalid/?redirect=${encodeURIComponent(redirectUri)}&state=${state}`,
    open:async address=>{
      const auth=new URL(address),redirect=new URL(auth.searchParams.get("redirect"));
      redirect.search="?code=ignored&state=wrong";assert.equal((await fetch(redirect)).status,400);
      redirect.search=`?code=accepted&state=${auth.searchParams.get("state")}`;
      const response=await fetch(redirect);assert.equal(response.status,200);assert.equal(response.headers.get("referrer-policy"),"no-referrer");assert.doesNotMatch(await response.text(),/accepted/);
    }});
  assert.equal(result.code,"accepted");assert.match(result.redirectUri,/^http:\/\/127\.0\.0\.1:\d+\/$/);
});

test("native shared worker attaches bytes, exports portable archives, restores independently, and saves files",async t=>{
  const host=await nativeHost(document());t.after(()=>host.close());
  const source=join(host.root,"source.txt"),archive=join(host.root,"backup.json"),saved=join(host.root,"restored.txt");
  writeFileSync(source,"native file contents");
  await host.request({kind:"attach",task_id:2,path:source,name:"source.txt",mime_type:"text/plain"});
  const oldKey=host.state.document.tasks[0].attachments[0].storage_key;
  await host.request({kind:"exportArchive",path:archive});
  const backup=JSON.parse(readFileSync(archive));assert.equal(backup.kind,"ferroui.todo.backup");assert.equal(Buffer.from(backup.files[0].base64,"base64").toString(),"native file contents");
  await host.request({kind:"import",path:archive,expected:JSON.stringify(host.state.document)});
  const key=host.state.document.tasks[0].attachments[0].storage_key;assert.notEqual(key,oldKey);assert.equal(host.state.document.settings.auto_sync,false);
  await host.request({kind:"openAttachment",key,path:saved});assert.equal(readFileSync(saved,"utf8"),"native file contents");
  assert.equal(host.state.callbacks.filter(value=>value==="importData").length,1);
  assert.ok(host.state.statuses.some(value=>value.includes("sync is paused")));
});

test("native restore rejects a changed UI revision and removes staged attachment bytes",async t=>{
  const host=await nativeHost(document());t.after(()=>host.close());
  const source=join(host.root,"source.txt"),archive=join(host.root,"backup.json");writeFileSync(source,"file");
  await host.request({kind:"attach",task_id:2,path:source,name:"source.txt"});await host.request({kind:"exportArchive",path:archive});
  const before=readdirSync(join(host.data,"files")).sort();
  host.state.beforeCallback=(message,state,persist)=>{
    if(message.method==="importData"){state.document.tasks[0].title="Keep the newer local edit";persist();}
  };
  await assert.rejects(host.request({kind:"import",path:archive,expected:JSON.stringify(host.state.document)}),/compare-and-swap/);
  assert.equal(host.state.document.tasks[0].title,"Keep the newer local edit");assert.deepEqual(readdirSync(join(host.data,"files")).sort(),before);
});

test("native restore cannot commit when durable sign-in revocation fails and rolls back staged files",async t=>{
  const configuration={clientId:"12345678-1234-1234-1234-123456789abc",tenant:"common",cloud:"global",redirectUri:"http://localhost/"};
  const host=await nativeHost(document(),{prepare:({data})=>{
    const key="ferroui.todo.native.public-configuration",sidecars=join(data,"service-state");
    mkdirSync(sidecars);
    writeFileSync(join(sidecars,`${keyName(key)}.json`),JSON.stringify({key,value:JSON.stringify(configuration)}));
  }});
  t.after(()=>host.close());
  const source=join(host.root,"source.txt"),archive=join(host.root,"backup.json");
  writeFileSync(source,"Preserve this existing attachment");
  await host.request({kind:"attach",task_id:2,path:source,name:"source.txt"});
  await host.request({kind:"exportArchive",path:archive});
  const backup=JSON.parse(readFileSync(archive));
  backup.document.tasks[0].title="This import must never replace the document";
  writeFileSync(archive,JSON.stringify(backup));
  const expected=JSON.stringify(host.state.document),folder=join(host.data,"files");
  const before=readdirSync(folder).sort();
  const beforeContents=before.map(name=>readFileSync(join(folder,name)));
  const beforeImports=host.state.callbacks.filter(value=>value==="importData").length;
  const marker=join(host.data,"secure-signin",`${cacheScope(host.data,configuration).id}.json`);
  let observedStagedFiles=false;
  host.state.beforeCallback=message=>{
    if(!observedStagedFiles&&message.method==="exportData"&&readdirSync(folder).length>before.length){
      observedStagedFiles=true;
      // A directory at the atomic-write destination reliably refuses rename,
      // including when tests run as root. The real worker must revoke its
      // saved sign-in before sending the final importData callback.
      mkdirSync(marker);
    }
  };
  await assert.rejects(host.request({kind:"import",path:archive,expected}),/rename|directory|access/i);
  assert.equal(observedStagedFiles,true,"the real portable pipeline must stage file bytes before this failure");
  assert.equal(host.state.callbacks.filter(value=>value==="importData").length,beforeImports);
  assert.equal(JSON.stringify(host.state.document),expected);
  assert.equal(readFileSync(join(host.data,"tasks.json"),"utf8"),expected);
  assert.deepEqual(readdirSync(folder).sort(),before);
  assert.deepEqual(before.map(name=>readFileSync(join(folder,name))),beforeContents);
});

test("native local attachments remain recoverable for undo and missing accounts never invent sessions",async t=>{
  const host=await nativeHost(document());t.after(()=>host.close());const source=join(host.root,"a.txt"),output=join(host.root,"saved.txt");writeFileSync(source,"undo");
  await host.request({kind:"attach",task_id:2,path:source,name:"a.txt"});const key=host.state.document.tasks[0].attachments[0].storage_key;
  await host.request({kind:"deleteAttachment",key});await host.request({kind:"openAttachment",key,path:output});assert.equal(readFileSync(output,"utf8"),"undo");
  await assert.rejects(host.request({kind:"syncMicrosoft",today:"2026-11-22"}),/Connect your Microsoft account/);
  await assert.rejects(host.request({kind:"connectMicrosoft",client_id:"not-an-id"}),/GUID/);
  await host.request({kind:"disconnectMicrosoft"});
});

test("native shared PST export writes a real Outlook store with file content",async t=>{
  const host=await nativeHost(document());t.after(()=>host.close());
  const source=join(host.root,"a.txt"),output=join(host.root,"tasks.pst");writeFileSync(source,"PST file contents");
  await host.request({kind:"attach",task_id:2,path:source,name:"a.txt"});
  await host.request({kind:"exportPst",path:output});
  const bytes=readFileSync(output);assert.equal(bytes.readUInt32LE(0),0x4e444221);assert.ok(bytes.length>512);assert.ok(host.state.statuses.some(value=>value.includes("Outlook tasks exported")));
});
