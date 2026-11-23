import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";

const button = label => element => element.type.endsWith("Button") && element.automationName === label;
const named = name => element => element.name === name;

// Runs after the other reviews: restore intentionally replaces the live test
// document. Every mutation uses actual controls, downloads, and file input.
export async function verifyPstRoundTrip(ui, output) {
  const { page, saved, snapshot, navigate, resize, control, clickControl, clickText, edit, until, capture } = ui;
  const directory=path.resolve(output,"pst-roundtrip");
  await mkdir(directory,{recursive:true});
  const downloads=await mkdtemp(path.join(directory,"downloads-"));
  const filename="pst-ui-attachment.txt", contents=Buffer.from("PST file round trip\nZażółć — UTF-8 bytes ✓\n");
  const sourceFile=path.join(directory,filename);
  await writeFile(sourceFile,contents);
  let chooser, listening=true, failure;
  page.on("Page.fileChooserOpened",event=>{if(listening)chooser=event;});
  const command=async(method,parameters,browser=false)=>{
    const result=await (browser?page.browserSend(method,parameters):page.send(method,parameters));
    assert(!result.error,`${method}: ${result.error?.message}`);return result;
  };
  const visible=async(match,label,scrollPoint={x:1200,y:600})=>{
    for(let attempt=0;attempt<12;attempt++) {
      const elements=(await snapshot()).elements;
      if(elements.some(element=>element.hit&&match(element))) return control(match,label);
      const target=elements.find(match);
      await page.wheel(scrollPoint.x,scrollPoint.y,0,target&&target.y<90?-260:260);
    }
    return control(match,label);
  };
  const choose=async(match,label,file)=>{
    chooser=null;
    await clickControl(match,label);
    await until(()=>chooser,`${label} opens the browser file chooser`);
    assert(chooser.backendNodeId,`${label} must use an actual file input`);
    await command("DOM.setFileInputFiles",{backendNodeId:chooser.backendNodeId,files:[file]});
  };
  const downloaded=async name=>{
    const file=path.join(downloads,name);
    await until(async()=>{
      try{return (await stat(file)).size>0;}catch(error){if(error.code==="ENOENT")return false;throw error;}
    },`Chrome completes the ${name} download`,20_000);
    return {file,bytes:await readFile(file)};
  };
  const openSettings=async()=>{
    await clickControl(button("Settings"),"PST Settings");
    await visible(button("Export Outlook PST"),"PST export action");
  };
  try {
    await command("Browser.setDownloadBehavior",{behavior:"allow",downloadPath:downloads,eventsEnabled:true},true);
    await command("Page.setInterceptFileChooserDialog",{enabled:true});
    await navigate("Page.navigate",{url:page.url}); // Leave demo mode using ordinary navigation.
    await resize(1280,800);
    await clickText("New list");
    await edit("ListName","PST round trip",true);
    const list=await until(async()=>(await saved()).lists.find(list=>list.name==="PST round trip"),"PST list creation");
    const title="PST UI task — Zażółć", stepTitle="PST UI step", notes="A real Settings export and import ✓\nKeep the note and file.";
    await edit("NewTask",title,true);
    const initial=await until(async()=>(await saved()).tasks.find(task=>task.title===title),"PST task creation");
    const task=async()=>(await saved()).tasks.find(task=>task.id===initial.id);
    await clickControl(element=>element.type.endsWith("ToggleButton")&&element.automationName==="Mark as important","mark PST task important");
    await clickText(title);
    await edit("NewStep",stepTitle,true);
    await clickControl(element=>element.type.endsWith("CheckBox")&&element.automationName===`Mark ${stepTitle} as completed`,"complete PST step");
    await clickControl(named("TaskMyDay"),"add PST task to My Day");
    await clickControl(named("TaskDueDate"),"PST due date");
    await clickControl(element=>element.type.endsWith("MenuItem")&&element.automationName?.startsWith("Tomorrow"),"tomorrow due date");
    await clickControl(named("TaskRepeat"),"PST repeat schedule");
    await clickControl(element=>element.type.endsWith("MenuItem")&&element.automationName==="Weekly","weekly repeat schedule");
    await visible(named("TaskNotes"),"PST note");
    // CDP insertText is delivered through the browser text-input bridge, whose
    // single-line composing surface normalizes embedded line breaks. Exercise
    // the editor's real Enter handling, as the detail review does, then commit.
    const [firstLine,secondLine]=notes.split("\n");
    await edit("TaskNotes",firstLine);
    await page.press("Enter");
    await page.type(secondLine);
    await page.press("Enter",["ctrl"]);
    await until(async()=>(await task()).notes===notes,"multiline PST note is committed");
    await visible(named("AddAttachment"),"PST file action");
    await choose(named("AddAttachment"),"attach PST fixture",sourceFile);
    await until(async()=>(await task()).attachments.some(file=>file.name===filename),"PST attachment storage");
    await page.press("Escape");
    await control(named("NewTask"),"PST list after closing details");
    const before=await saved(), source=before.tasks.find(task=>task.id===initial.id);
    assert.equal(source.list_id,list.id);assert.equal(source.notes,notes);
    assert.equal(source.steps[0].completed,true);assert.equal(source.recurrence?.unit,"Week");
    assert.equal(source.important,true);assert(source.my_day);

    await openSettings();
    await clickControl(button("Export Outlook PST"),"export real Outlook PST");
    const archive=await downloaded("todo-outlook-tasks.pst");
    assert.equal(archive.bytes.subarray(0,4).toString("ascii"),"!BDN","download contains authentic PST bytes");
    assert.equal(archive.bytes.readUInt16LE(10),23,"download is a Unicode PST");
    await clickControl(button("Close settings"),"close PST export Settings");
    await until(async()=>(await snapshot()).elements.some(element=>element.text?.startsWith("Outlook PST downloaded.")),"visible PST export result");

    // An actual intervening edit proves restore consumed the downloaded file,
    // rather than simply observing the untouched fixture already in the app.
    await clickText(title);
    await edit("TaskTitle","Edited after PST export",true);
    await page.press("Escape");
    await edit("NewTask","Only after PST export",true);
    await until(async()=>(await saved()).tasks.length===before.tasks.length+1,"post-export edit is saved");
    await openSettings();
    await choose(button("Import Outlook PST…"),"import downloaded Outlook PST",archive.file);
    const restored=await until(async()=>{
      const document=await saved();
      return document.tasks?.some(task=>task.title===title)
        && !document.tasks.some(task=>["Edited after PST export","Only after PST export"].includes(task.title)) && document;
    },"PST worker parses and commits the downloaded document",30_000);
    assert.equal(restored.tasks.length,before.tasks.length,"restore replaces the exported task collection exactly once");
    const imported=restored.tasks.find(task=>task.title===title);
    assert.equal(restored.lists.find(list=>list.id===imported.list_id)?.name,"PST round trip");
    for(const field of ["notes","completed","important","due_date","recurrence","my_day"]) assert.deepEqual(imported[field],source[field],`PST ${field} survives the UI flow`);
    assert.deepEqual(imported.steps.map(({title,completed})=>({title,completed})),source.steps.map(({title,completed})=>({title,completed})),"structured steps survive PST private metadata");
    assert.equal(imported.attachments.length,1);
    assert.equal(imported.attachments[0].name,filename);
    assert.equal(imported.attachments[0].size,contents.length);
    assert.notEqual(imported.attachments[0].storage_key,source.attachments[0].storage_key,"restore assigns an independent local file key");
    for(const field of ["auto_sync","planner_enabled","outlook_categories_enabled","presentation_sync_enabled","keep_running_in_background"]) assert.equal(restored.settings[field],false,`${field} remains paused after restore`);
    assert(restored.lists.every(list=>!list.cloud),"restored lists have no active cloud mapping");
    assert(restored.tasks.every(task=>!task.external&&!task.flagged_email&&!task.assigned_to),"restored tasks are independent local copies");
    await until(async()=>(await snapshot()).elements.some(element=>element.text?.includes("Restored as a local copy")),"visible independent-copy restore status");
    if((await snapshot()).elements.some(element=>element.hit&&button("Close settings")(element)))await clickControl(button("Close settings"),"close PST import Settings");
    await navigate("Page.reload",{ignoreCache:true});
    assert.deepEqual(await saved(),restored,"restored PST state survives a real reload");
    // Previous reviews retain their documents on failure. A longer sidebar
    // requires ordinary scrolling before this exact list can receive a click.
    await visible(button("PST round trip"),"restored PST list",{x:140,y:600});
    await clickControl(button("PST round trip"),"restored PST list");
    await clickText(title);
    await visible(button(filename),"restored PST attachment");
    await clickControl(button(filename),"download restored attachment");
    assert.deepEqual((await downloaded(filename)).bytes,contents,"the restored attachment opens with its original bytes");
    await capture("pst-roundtrip-restored");
    console.log("ok: real Settings PST download/import, Unicode task/list/steps/repeat, independent keys and paused sync, attachment bytes, and reload persistence");
  } catch(error) {
    failure=error;
    throw error;
  } finally {
    listening=false;
    const cleanup=await Promise.allSettled([
      command("Page.setInterceptFileChooserDialog",{enabled:false}),
      command("Browser.setDownloadBehavior",{behavior:"default"},true),
    ]);
    const errors=cleanup.filter(result=>result.status==="rejected").map(result=>result.reason);
    if(errors.length&&!failure)throw new AggregateError(errors,"PST browser download/chooser cleanup failed");
  }
}
