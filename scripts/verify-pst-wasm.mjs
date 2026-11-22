// Small post-build smoke check: real PST import, emitted memory ceiling, and a
// malicious multi-value count. The malicious case must never run in-process
// through the native reference reader; WASM confines its allocation failure.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { fixturePst } from "./tests/pst-fixture.mjs";
import { pstParserError, PST_RESOURCE_ERROR } from "../crates/todo-browser/wwwroot/backup/pst-errors.js";

const directory=resolve(process.argv[2] || "target/browser/todo-browser/interop");
const bytes=await readFile(`${directory}/todo_interop_wasm_bg.wasm`);
let position=8, maximum;
function leb() {
  let value=0,shift=0,byte;
  do {byte=bytes[position++];value+=(byte&127)*2**shift;shift+=7;} while(byte&128);
  return value;
}
while(position<bytes.length) {
  const id=bytes[position++],size=leb(),end=position+size;
  if(id===5) {
    assert.equal(leb(),1,"The PST worker must have exactly one linear memory");
    assert.equal(leb(),1,"The PST worker memory must declare a maximum");
    leb();maximum=leb();break;
  }
  position=end;
}
assert.equal(maximum,8192,"The PST parser must be capped at 512 MiB");
const {default:initialize,importPstData}=await import(pathToFileURL(`${directory}/todo_interop_wasm.js`));
await initialize({module_or_path:bytes});
const current=JSON.stringify({schema_version:1,tasks:[],lists:[{id:1,name:"Tasks"}],groups:[],settings:{},next_id:2});
const fixture=await readFile(new URL("../crates/todo-interop/tests/fixtures/tasks.pst",import.meta.url));
const result=JSON.parse(importPstData(fixture,current,"2026-11-22"));
assert.equal(result.archive.document.tasks.length,2);
assert.equal(result.archive.files.length,2);
assert.equal(result.skipped_messages,1);
const malicious=new Uint8Array(await fixturePst({malformedCategories:true}).arrayBuffer());
assert.throws(()=>importPstData(malicious,current,"2026-11-22"),error=>pstParserError(error)===PST_RESOURCE_ERROR);
console.log("PST WASM verified: real task/file import, 512 MiB memory cap, malformed-count trap.");
