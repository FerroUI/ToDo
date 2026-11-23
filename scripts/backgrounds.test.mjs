import test from "node:test";
import assert from "node:assert/strict";
import { validBackground } from "../crates/todo-browser/wwwroot/backgrounds.js";
import { validFieldValue, validateReplica, fieldKey } from "../crates/todo-browser/wwwroot/cloud/presentation-format.js";

const jpeg = Uint8Array.of(0xff,0xd8,0xff,0xc0,0,8,8,0,32,0,64,1,0xff,0xd9);
const uri = bytes => `data:image/jpeg;base64,${Buffer.from(bytes).toString("base64")}`;
test("custom backgrounds reject oversized dimensions, wrong formats and invalid encoding before native decoding", () => {
  assert(validBackground(uri(jpeg))); assert(validBackground("scene:mountains"));
  const huge=jpeg.slice();huge[9]=0xff;huge[10]=0xff;
  assert(!validBackground(uri(huge))); assert(!validBackground(uri(jpeg.slice(0,9))));
  assert(!validBackground(`data:image/svg+xml,<svg/>`)); assert(!validBackground(`${uri(jpeg)}!`));
  assert(!validBackground(uri(new Uint8Array(192*1024+1))));
});
test("OneDrive presentation replicas preserve portable wallpapers for lists and smart views", () => {
  const image=uri(jpeg), device="12345678-1234-4234-9234-123456789abc";
  assert(validFieldValue(["list","list","background"],image));
  assert(validFieldValue(["settings","app","smart_backgrounds"],{"My Day":image,Important:"scene:forest"}));
  const replica={kind:"ferroui.todo.presentation",version:1,device,records:{
    [fieldKey("list","list","background")]:{clock:[1,device],deleted:false,value:image},
  }};
  assert.deepEqual(validateReplica(JSON.parse(JSON.stringify(replica))),replica);
  replica.records[fieldKey("list","list","background")].value="data:text/html,unsafe";
  assert.throws(()=>validateReplica(replica),/invalid/);
});
