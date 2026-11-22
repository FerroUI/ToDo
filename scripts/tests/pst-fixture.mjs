import { MemorySink, PstStore, setRandom } from "../../crates/todo-browser/wwwroot/vendor/pst-writer/index.js";
import { taskNames, taskProperties } from "../../crates/todo-browser/wwwroot/backup/pst-properties.js";
import { fileTime } from "../../crates/todo-browser/wwwroot/vendor/pst-writer/bytes.js";
import { TASK } from "../../crates/todo-browser/wwwroot/vendor/pst-writer/names.js";
import { BODY_TIME_SET } from "../../crates/todo-browser/wwwroot/backup/pst-source.js";

export function fixturePst({ malformedCategories = false } = {}) {
  setRandom(length => Uint8Array.from({length}, (_,index)=>index+1));
  try {
    const sink=new MemorySink(),store=new PstStore(sink,"Outlook tasks fixture",taskNames);
    const task={title:"Review design — Zażółć",notes:"A plain note\nSecond line.",important:true,completed:true,
      completed_on:"2026-11-20",due_date:"2026-11-19",reminder:{date:"2026-11-18",minutes_after_midnight:615},
      tags:["Project, A","Żółty"],steps:[{title:"Review section",completed:true}],my_day:"2026-11-22",
      recurrence:{unit:"Week",every:2,anchor:"2026-11-19"}};
    const message={subject:task.title,body:task.notes,senderName:"",senderEmail:"",displayTo:"",
      received:new Date("2026-11-01T12:00:00Z"),sent:new Date("2026-11-01T12:00:00Z"),recipients:[],attachments:[]};
    const properties=taskProperties(task);
    if(malformedCategories) {
      // A structurally valid property stream with an impossible 2^27-element
      // count: execute only in capped WASM or an allocation-bounded native child.
      properties.find(value=>value.id===0x8000).data=Uint8Array.of(0,0,0,8);
    }
    store.addMessage(store.folder("Tasks","IPF.Task"),{...message,messageClass:"IPM.Task",
      attachments:[{name:"notes.txt",mime:"text/plain",data:new TextEncoder().encode("file bytes ✓")},
        {name:"multi-block.bin",mime:"application/octet-stream",data:Uint8Array.from({length:9000},(_,index)=>index%251)}],extraProperties:properties});
    store.addMessage(store.folder("Tasks","IPF.Task"),{...message,messageClass:"IPM.Task",subject:"Plain Outlook task",body:"No application metadata",
      extraProperties:taskProperties({...task,completed:false,important:false,reminder:null,tags:[],due_date:null}).filter(value=>value.id!==0x8002+taskNames.length-1)});
    store.folder("Empty task list","IPF.Task");
    store.addMessage(store.folder("Inbox"),{...message,subject:"Not a task",body:"Do not import me."});
    store.finish();return sink.blob();
  } finally {setRandom(length=>crypto.getRandomValues(new Uint8Array(length)));}
}

export function fidelityPst() {
  setRandom(length=>Uint8Array.from({length},(_,index)=>index+1));
  try {
    const sink=new MemorySink(),store=new PstStore(sink,"Preserved source task fields",taskNames);
    const source={importance:"low",startDateTime:{dateTime:"2026-10-28T00:00:00",timeZone:"UTC"},
      createdDateTime:"2026-10-01T12:30:45.123Z",lastModifiedDateTime:"2026-11-01T13:20:10Z",
      body:{contentType:"text",content:"Source note"},pst:{properties:{
        af01:{type:"Floating64",value:0.375},af02:{type:"Time",value:fileTime(new Date("2026-11-01T13:00:00Z")).toString()},
      },namedProperties:{af01:{guid:TASK,name:"00008102"},af02:{guid:BODY_TIME_SET,name:"BodyLastModifiedTime"}}}};
    for(const status of ["inProgress","waitingOnOthers","deferred","editedCompleted","editedNormal"]) {
      const task={title:status,notes:"Source note",completed:status==="editedCompleted",important:status==="editedCompleted",
        due_date:"2026-11-20",created_on:"2026-10-01",updated_on:"2026-11-01",steps:[],attachments:[],tags:[],
        completed_on:status==="editedCompleted"?"2026-11-01":null,
        microsoft_data:{...source,status:status==="editedCompleted"?"inProgress":status==="editedNormal"?"completed":status,
          importance:status==="editedNormal"?"high":"low"}};
      store.addMessage(store.folder("Tasks","IPF.Task"),{messageClass:"IPM.Task",subject:task.title,body:task.notes,
        senderName:"",senderEmail:"",displayTo:"",received:new Date("2026-10-01T00:00:00Z"),sent:new Date("2026-11-01T00:00:00Z"),
        recipients:[],attachments:[],extraProperties:taskProperties(task)});
    }
    store.finish();return sink.blob();
  }finally{setRandom(length=>crypto.getRandomValues(new Uint8Array(length)));}
}
