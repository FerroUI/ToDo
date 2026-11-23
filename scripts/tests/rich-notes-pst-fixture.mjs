import { MemorySink, PstStore, setRandom } from "../../crates/todo-browser/wwwroot/vendor/pst-writer/index.js";
import { taskNames, taskProperties } from "../../crates/todo-browser/wwwroot/backup/pst-properties.js";
import { richNoteFields } from "../../crates/todo-browser/wwwroot/notes.js";

export function richNotesPst() {
  setRandom(length=>Uint8Array.from({length},(_,index)=>index+1));
  try {
    const sink=new MemorySink(),store=new PstStore(sink,"Rich notes",taskNames);
    const note=richNoteFields('<p><strong>日本語 #Release</strong> <a href="https://example.com/">link</a></p><ul><li>Review 👩🏽‍💻</li></ul>');
    const task={...note,completed:false,important:false,steps:[{title:"Check",completed:false}],tags:[]};
    const body=`${note.notes}\n\nSteps:\n[ ] Check`,html=`${note.notes_html}<p style="white-space:pre-wrap"></p><p style="white-space:pre-wrap">Steps:</p><p style="white-space:pre-wrap">[ ] Check</p>`;
    for(const edited of [false,true]) {
      store.addMessage(store.folder("Tasks","IPF.Task"),{messageClass:"IPM.Task",subject:edited?"Outlook formatting edit":"Rich note roundtrip",body,
        html:edited?html.replaceAll("strong>","em>"):html,senderName:"",senderEmail:"",displayTo:"",received:new Date("2026-11-01T00:00:00Z"),sent:new Date("2026-11-01T00:00:00Z"),recipients:[],attachments:[],
        extraProperties:taskProperties(task,body,html)});
    }
    store.finish();return sink.blob();
  } finally { setRandom(length=>crypto.getRandomValues(new Uint8Array(length))); }
}
