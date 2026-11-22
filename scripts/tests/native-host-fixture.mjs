import { mkdtempSync, cpSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";

const repository = fileURLToPath(new URL("../../", import.meta.url));

export async function nativeHost(document, { prepare } = {}) {
  const root = mkdtempSync(join(tmpdir(), "ferroui-native-test-"));
  const code = join(root, "runtime"), data = join(root, "data");
  mkdirSync(code); mkdirSync(data);
  cpSync(join(repository, "crates/todo-desktop/native"), join(code,"native"), { recursive: true });
  for (const path of ["cloud", "backup", "vendor/pst-writer"]) cpSync(join(repository,"crates/todo-browser/wwwroot",path), join(code,"web",path), { recursive: true });
  writeFileSync(join(code,"package.json"), '{"type":"module"}');
  const state = { document: structuredClone(document), statuses: [], callbacks: [], beforeCallback: null };
  const persist = () => writeFileSync(join(data,"tasks.json"), JSON.stringify(state.document));
  persist();
  await prepare?.({ root, code, data });
  const environment = { ...process.env };
  for (const key of Object.keys(environment)) if (key.startsWith("TODO_MICROSOFT_") || ["NODE_OPTIONS","NODE_PATH"].includes(key)) delete environment[key];
  const processHandle = spawn(process.execPath, [join(code,"native/worker.mjs")], { stdio: ["pipe","pipe","pipe"], env: environment });
  let buffer = Buffer.alloc(0), nextId = 0, stderr = "";
  const pending = new Map();
  let readyResolve, readyReject;
  const ready = new Promise((resolve,reject) => { readyResolve=resolve;readyReject=reject; });
  const send = value => {
    const bytes=Buffer.from(JSON.stringify(value)), header=Buffer.alloc(4);header.writeUInt32LE(bytes.length);
    processHandle.stdin.write(header);processHandle.stdin.write(bytes);
  };
  processHandle.stderr.on("data", value => { stderr += value; });
  processHandle.on("exit", code => {
    const error = new Error(`Native worker exited ${code}: ${stderr.slice(-2000)}`);
    readyReject(error);for(const request of pending.values()) request.reject(error);pending.clear();
  });
  const callback = async message => {
    try {
      await state.beforeCallback?.(message, state, persist);
      const { method, args }=message;state.callbacks.push(method);
      let value=null;
      if(method==="exportData") value=JSON.stringify(state.document);
      else if(method==="validateImportData"||method==="prepareCloudData") value=JSON.stringify(JSON.parse(args.json));
      else if(method==="importData"||method==="replaceFromCloud") {
        if(args.expected!==JSON.stringify(state.document)) throw new Error("Tasks changed during native compare-and-swap.");
        state.document=JSON.parse(args.json);persist();
      } else if(method==="addAttachment") {
        const task=state.document.tasks.find(task=>task.id===args.task_id);
        if(!task) throw new Error("Task no longer exists.");
        task.attachments.push({...args.attachment,id:state.document.next_id++});persist();
      } else if(method==="setIntegrationPreference") {state.document.settings[args.feature]=args.enabled;persist();}
      else if(method==="hostStatus"||method==="hostError") state.statuses.push(args.message);
      else if(method==="openUrl"||method==="openExternalUrl") throw new Error("Tests never open a Microsoft sign-in session.");
      send({type:"callbackResult",id:message.id,value});
    } catch(error) { send({type:"callbackResult",id:message.id,error:error.message}); }
  };
  processHandle.stdout.on("data", chunk => {
    buffer=Buffer.concat([buffer,chunk]);
    while(buffer.length>=4&&buffer.length>=4+buffer.readUInt32LE()) {
      const length=buffer.readUInt32LE(), message=JSON.parse(buffer.subarray(4,4+length));buffer=buffer.subarray(4+length);
      if(message.type==="ready")readyResolve();
      else if(message.type==="callback")void callback(message);
      else if(message.type==="result") {
        const request=pending.get(message.id);pending.delete(message.id);
        if(!request) throw new Error("Unexpected native result");
        message.error?request.reject(new Error(message.error)):request.resolve(message.value);
      }
    }
  });
  send({type:"initialize",directory:data,demo:false});
  await ready;
  const request=action=>new Promise((resolve,reject)=>{const id=++nextId;pending.set(id,{resolve,reject});send({type:"command",id,action});});
  await request({kind:"initialize"});
  return { state, root, data, request, persist, close: async()=>{
    const done=new Promise(resolve=>processHandle.once("exit",resolve));processHandle.kill();await done;rmSync(root,{recursive:true,force:true});
  }};
}
