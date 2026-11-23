import test from "node:test";
import assert from "node:assert/strict";
import { noteFields, noteBody, parseNoteHtml, richNoteFields, writeNoteHtml, safeNoteLink } from "../crates/todo-browser/wwwroot/notes.js";
import { fields, remoteFields, mergeFields, patchFor } from "../crates/todo-browser/wwwroot/cloud/codec.js";

test("rich-note HTML preserves Unicode, whitespace, lists, emphasis and safe links", () => {
  const html="<p><strong>日本語 👩🏽‍💻</strong> <em>é</em><br><u><s>done</s></u> <a href='https://example.com/a?x=1&amp;y=2'>site</a></p><ol start='3'><li>three</li><li>four</li></ol><ul><li>item</li></ul>";
  const note=richNoteFields(html);
  assert.equal(note.notes,"日本語 👩🏽‍💻 é\ndone site\n3. three\n4. four\n• item");
  assert.deepEqual(richNoteFields(note.notes_html),note);
  const text="  leading\n\nA < B & C\ntrailing  \n";
  assert.equal(remoteFields({body:noteBody({notes:text})}).notes,text);
});

test("notes emit only owned markup and reject active content, bad links and excessive depth", () => {
  const note=richNoteFields("<script>alert(1)</script><p onclick='x()'><img src=x onerror=x()>safe <a href='&#106;avascript:alert(1)'>bad</a><a href='https://user:pass@example.com'>credentials</a><span style='font-weight:700;text-decoration:line-through;background:url(x)'>strong</span></p><svg><text>hidden</text></svg>");
  assert.equal(note.notes,"safe badcredentialsstrong");
  assert.equal(note.notes_html,'<p style="white-space:pre-wrap">safe badcredentials<strong><s>strong</s></strong></p>');
  for(const url of ["javascript:alert(1)","data:text/html,x","https://a@evil.test","https://good.test/\nno","https://good.test\\@evil.test"]) assert.equal(safeNoteLink(url),null);
  assert.throws(()=>parseNoteHtml("<span>".repeat(130)),/nested/);
  assert.throws(()=>parseNoteHtml("é".repeat(1024*1024)),/1 MiB/);
  assert.equal(writeNoteHtml(parseNoteHtml("<p title='one > two'><b>ok</b></p>")),'<p style="white-space:pre-wrap"><strong>ok</strong></p>');
});

test("Graph merges text and formatting atomically and uploads formatting-only edits", () => {
  const base=fields({title:"Task",notes:"Hello"}), bold=fields({title:"Task",...richNoteFields("<b>Hello</b>")});
  assert.deepEqual(patchFor(base,base,"2026-11-23"),{});
  assert.match(patchFor(bold,base,"2026-11-23").body.content,/<strong>Hello<\/strong>/);
  const cleared=patchFor(base,bold,"2026-11-23");
  assert.ok(cleared.body); assert.ok(!cleared.body.content.includes("strong"));
  assert.deepEqual(mergeFields(base,bold,base,base,"Task"),bold);
  const remote=fields({title:"Task",notes:"Changed remotely"});
  assert.throws(()=>mergeFields(bold,remote,base,base,"Task"),/conflict.*notes/i);
  assert.deepEqual(noteFields({notes:"Legacy edit",notes_html:bold.notes_html}),{notes:"Legacy edit",notes_html:null});
});

test("legacy Graph journals stay unchanged for ordinary text and upgrade on a rich edit", () => {
  const old=fields({title:"Task",notes:"plain"});
  assert.ok(!Object.hasOwn(old,"notes_html"));
  const remote=remoteFields({title:"Task",body:{contentType:"html",content:"plain"}});
  assert.deepEqual(mergeFields(old,remote,old,remote,"Task"),old);
  const next=fields({title:"Task",...richNoteFields("<em>plain</em>")});
  const result=mergeFields(next,remote,old,remote,"Task");
  assert.equal(result.notes_html,next.notes_html);
});
