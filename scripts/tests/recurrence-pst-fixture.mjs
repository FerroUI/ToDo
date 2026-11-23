import { MemorySink, PstStore, setRandom } from "../../crates/todo-browser/wwwroot/vendor/pst-writer/index.js";
import { taskNames, taskProperties } from "../../crates/todo-browser/wwwroot/backup/pst-properties.js";
import { TASK } from "../../crates/todo-browser/wwwroot/vendor/pst-writer/names.js";
import { prop } from "../../crates/todo-browser/wwwroot/vendor/pst-writer/ltp.js";
import { encodePstRecurrence } from "../../crates/todo-browser/wwwroot/backup/pst-recurrence.js";

export const recurrenceCases = [
  ["Daily", { unit: "Day", every: 3, anchor: "2026-11-23", end: { kind: "after", value: 4 } }],
  ["Weekly set", { unit: "Week", every: 2, anchor: "2026-11-23", days_of_week: ["monday", "thursday"], week_start: "monday", end: { kind: "on", value: "2026-12-31" } }],
  ["Month end", { unit: "Month", every: 1, anchor: "2027-01-31", end: { kind: "after", value: 4 } }],
  ["Leap year", { unit: "Year", every: 1, anchor: "2024-02-29", end: { kind: "after", value: 3 } }],
  ["Last selected weekday", { unit: "Month", every: 1, anchor: "2026-11-30", days_of_week: ["monday", "friday"], ordinal: "last", relative_mode: "nthMatchingDay", end: { kind: "after", value: 3 } }],
  ["Relative yearly", { unit: "Year", every: 1, anchor: "2026-11-26", days_of_week: ["thursday"], ordinal: "fourth", end: { kind: "after", value: 4 } }],
];
const id = lid => 0x8002 + taskNames.findIndex(value => value.guid === TASK && value.lid === lid);

export function recurrencePst() {
  setRandom(length => Uint8Array.from({ length }, (_, index) => index + 1));
  try {
    const sink = new MemorySink(), store = new PstStore(sink, "Recurrence", taskNames);
    function add(title, recurrence, { metadata = false, edit = null, due = recurrence?.anchor, completed = false, generated = false, start = null } = {}) {
      const task = { recurrence, due_date: due, start_date: start, completed, recurrence_generated: generated, created_on: "2026-11-01", updated_on: "2026-11-01", steps: [], notes: "", tags: [] };
      let properties = taskProperties(task);
      if (!metadata) properties = properties.filter(value => value.id !== 0x8002 + taskNames.length - 1);
      if (edit === "disable") properties = properties.map(value => value.id === id(0x8126) ? prop.bool(value.id, false) : value);
      if (edit === "change") properties = properties.map(value => value.id === id(0x8116) ? prop.binary(value.id, encodePstRecurrence({ ...task, recurrence: { unit: "Day", every: 5, anchor: due } }).data) : value);
      if (edit === "add") {
        properties = properties.map(value => value.id === id(0x8126) ? prop.bool(value.id, true) : value);
        properties.push(prop.binary(id(0x8116), encodePstRecurrence({ ...task, recurrence: { unit: "Day", every: 5, anchor: due } }).data), prop.bool(id(0x8109), false));
      }
      if (edit === "sliding") properties = properties.map(value => {
        if (value.id !== id(0x8116)) return value;
        const data = value.data.slice(); new DataView(data.buffer).setUint32(18, 1, true); return { ...value, data };
      });
      store.addMessage(store.folder("Tasks", "IPF.Task"), { messageClass: "IPM.Task", subject: title, body: "", senderName: "", senderEmail: "", displayTo: "", received: new Date("2026-11-01T00:00:00Z"), sent: new Date("2026-11-01T00:00:00Z"), recipients: [], attachments: [], extraProperties: properties });
    }
    for (const [name, rule] of recurrenceCases) add(name, rule);
    const rich = { unit: "Year", every: 3, anchor: "2024-01-01", month: 11, day_of_month: 15, end: { kind: "on", value: "2033-12-31" }, time_zone: "Pacific Standard Time" };
    add("Original range restored", rich, { metadata: true, due: "2027-11-15" });
    add("Outlook changed repeat", rich, { metadata: true, due: "2027-11-15", edit: "change" });
    add("Outlook disabled repeat", rich, { metadata: true, due: "2027-11-15", edit: "disable" });
    add("Sliding preserved", recurrenceCases[0][1], { edit: "sliding" });
    add("Remaining count", recurrenceCases[2][1], { due: "2027-02-28" });
    add("Completed occurrence", recurrenceCases[0][1], { completed: true });
    add("Outlook added repeat", null, { metadata: true, due: "2026-11-23", edit: "add" });
    add("App-only historical", { unit: "Month", every: 1, anchor: "2026-11-23", ordinal: "second", days_of_week: ["monday", "friday"] }, { metadata: true, completed: true, generated: true });
    add("Start without due", null, { metadata: true, start: "2026-12-01", due: null });
    store.finish(); return sink.blob();
  } finally { setRandom(length => crypto.getRandomValues(new Uint8Array(length))); }
}
