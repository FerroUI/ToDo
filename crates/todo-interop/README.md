# Outlook task interchange

`todo-interop` prepares independent local task data from Outlook PST files. It
never mutates the current document or a source PST. Browser and desktop hosts
use the same Rust parser, followed by the existing validated portable-archive
restore transaction.

```rust,ignore
let prepared = todo_interop::import_pst(&bytes, &current_document, today)?;
// Serialize prepared; normalize UTC reminders/completion dates in the host's
// timezone; validate and stage all files; then commit the independent document.
```

## Supported input

The parser uses Microsoft's MIT `outlook-pst` **1.2.0** reference implementation.
It reads ANSI PST versions 14/15 and Unicode PST version 23. It traverses the IPM
folder subtree and imports `IPM.Task` records, including subclassed tasks.
Non-task mail, calendar, contact, and other records are counted and skipped.
Empty `IPF.Task` folders are retained. A direct `Tasks` folder maps to the
application's default list; other task folders retain their path as list names.

Task status, importance, due dates, completion dates, reminders, category names,
plain text, and by-value file attachments are projected. Named properties are
resolved through the store's GUID/name/LID map, never by assuming that a LID is
a property ID. Original typed message properties and named-property identities
remain in `microsoft_data.pst`. HTML-only notes are converted to text by the
host using the same projection as Graph, while retaining original HTML.

UTC reminders and UTC completion instants are normalized using the host's
timezone at that date, including daylight-saving changes. Due/start dates use
Outlook's civil-date semantics; empty date sentinels are treated as absent.

The host-visible result contains `archive`, `warnings`, `skipped_messages`,
`utc_reminders`, and `utc_dates`. Warnings must remain visible after restore.
`normalizePstImport()` in the browser host performs the timezone projection;
native service workers reuse that function.

## Output

The shared browser/native service exporter writes a **new Unicode PST** with
`IPF.Task` folders and `IPM.Task` records. Standard fields include title, notes,
status/completion, importance, due date, reminder, categories, and actual file
bytes. Steps also appear in the note. Steps, My Day, source links, and local
repeat schedules are retained in versioned `FerroUI.ToDo.Task` named metadata
for restoration in this app. Outlook receives standalone task occurrences;
the exporter reports that repeat schedules are app metadata, not an Outlook
recurrence series.

Richer imported status, low importance, percent complete, and source start
dates survive export when the corresponding local projection is unchanged.
Local completion/importance edits take precedence. Invalidated start dates
(due date removed or moved earlier) are omitted with an explicit warning.
Precise source timestamps are retained; CommonStart/CommonEnd use UTC while
TaskStart/TaskDue retain civil dates.

This does not implement a general PST editor or full mailbox export. The writer
is an MIT `pst-writer` task extension; its source, pinned upstream revision, and
licenses are under `wwwroot/vendor/pst-writer/`. Synthetic output is checked
byte-for-byte and independently parsed with Microsoft's Rust implementation.
Desktop Outlook acceptance remains a separate manual verification.

## Limits and failure handling

Input/output PST: 256 MiB. File: 25 MiB. Total extracted file contents: 128 MiB.
Document metadata: 32 MiB. Traversal: 100,000 items, 10,000 folders, 64 levels.
The reader also bounds I/O amplification and rejects offsets outside the file,
truncated declared EOF, duplicate/cyclic folder or message identities, malformed
named-property maps, and unsupported format versions. Browser parsing runs in
a disposable worker with 512 MiB maximum linear memory and a 60-second
watchdog; cancellation terminates it. `import_pst_owned` avoids a second
complete input copy. Native callers must also isolate the parser and apply a
resource limit because upstream internal counts can request allocations before
projection limits run. See [the host contract](../../docs/OUTLOOK-PST.md).

OST/4K stores, corrupt stores, embedded Outlook-item attachments, and external
reference attachments are rejected. RTF-only notes and unsupported native
Outlook recurrence patterns are retained as original properties and reported
as not projected into the local editor. No PST input automatically publishes
to Microsoft. Restore still allocates new file keys, validates checksums,
detects concurrent edits, and disconnects synchronization before treating the
restored document as an independent copy.

## References

- [Microsoft PST reference implementation](https://github.com/microsoft/outlook-pst-rs)
- [MS-PST specification](https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/)
- [To Do-to-PST field mapping](https://learn.microsoft.com/en-us/graph/todo-concept-overview)
- [Task status semantics](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxotask/839a9304-3445-4f52-a879-833817e1dd5a)
- [Task due-date semantics](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxotask/19974e79-5f5b-4ab9-9584-1c555c7718d9)
- [Reminder properties](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxormdr/81bb1d6b-9416-4a1e-b8d9-2feedc237bfd)
