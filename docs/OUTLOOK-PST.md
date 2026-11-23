# Outlook PST task interchange

The app imports Outlook task records from a PST and exports the local task
projection as a new Unicode PST. This is an actual MS-PST binary interchange
path; the portable JSON backup and Microsoft Graph JSON export remain separate
formats. Use the portable backup when preserving the complete application
document is the priority.

## Import

`todo-interop` uses Microsoft's MIT-licensed `outlook-pst` 1.2.0 reference reader.
It accepts ANSI PST versions 14/15 and Unicode PST version 23. The upstream
reader supports the format's none, permute, and cyclic block encodings. These
encodings are not account authentication, and the importer does not sign into
Microsoft or upload anything. OST and version 36/4K stores are unsupported.

Only `IPM.Task` and its subclasses are imported. Mail, appointments, contacts,
and other item classes are counted and reported as skipped. A mailbox PST may
therefore contain many skipped items. Exporting only task folders from Outlook
is more efficient. An input with neither task records nor task folders is
rejected without changing the current document.

| Outlook data | Local result |
| --- | --- |
| Subject | Task title |
| Plain-text body | Searchable plain note; an explicit native plain-body edit takes precedence over a stale HTML alternative |
| HTML body | Bounded safe rich note with formatting, lists and links, plus a searchable plain projection; original properties retained |
| Task complete/status | Completed or active state; original status retained in source properties |
| Importance | High importance becomes Important; original value retained |
| Task start/due dates | Separate civil start and due dates |
| Native Gregorian recurrence | Daily, weekly weekday sets, absolute/relative monthly/yearly patterns and finite ranges; unsupported forms remain in source properties |
| Task completion date | Host-local date derived from the stored UTC completion instant |
| Reminder set/time | Host-local date and time, resolving the UTC instant using the offset at that date |
| Creation/modification/body modification times | Precise source timestamps retained alongside the local date projection |
| Categories | Category names |
| Task folders | Task lists; parent paths form list names; empty `IPF.Task` folders are retained |
| Direct `Tasks` folder | Built-in Tasks list |
| By-value file attachment | Actual file contents, name, MIME type, size, and checksum |
| Other typed MAPI fields | Retained under `microsoft_data.pst`; not all have local editor controls |

Named-property IDs are store specific. The reader resolves standard task LIDs
and named strings through each PST's GUID/name map. It does not assume that a
property ID equals a published MAPI LID. ANSI string properties use their
codepage; invalid text and malformed maps produce an error.

Supported native Outlook recurrence is projected into the shared recurrence
editor. Unsupported calendars, sliding rules, exceptions and future recurrence
structures remain in original properties with a warning. RTF-only bodies are
retained, but the editor does not decode arbitrary RTF layout. Embedded Outlook-item
attachments and external/linked attachments are rejected, because restoring
them as ordinary local files would misrepresent their contents. Export those
items as regular files in Outlook first.

The public API prepares a result without changing live application state:

```rust,ignore
let prepared = todo_interop::import_pst_owned(pst_bytes, &current, today)?;
// `import_pst(&[u8], ...)` is also available when ownership is inconvenient.
```

The result contains `archive`, `warnings`, `skipped_messages`, `utc_reminders`,
and `utc_dates`. `normalizePstImport()` projects UTC reminders/completion dates
into the host timezone before the portable restore transaction runs. Due dates
retain Outlook's civil-date semantics. Future reminder dates use the timezone
offset at that future date, including daylight-saving rules.

## Export

The shared browser/native exporter writes a new Unicode version 23 PST with
`IPF.Task` folders, `IPM.Task` records, the store's named-property map, and
by-value file attachments. Its task profile writes title, notes, completion,
importance, start/due dates, reminder, category names, and actual file bytes using
standard properties. It emits an unencoded PST and does not modify an input
PST in place. Export retains in-progress, waiting, or deferred status and low
importance from the imported Graph/PST source when the corresponding local
completion/Important state is unchanged. Changing those local controls takes
precedence. Preserved percent complete and precise creation/modification times
are also written where available.

Source start dates are preserved together with their required UTC companion
property. Outlook requires a due date on or after a start date. If a local edit
cleared the due date or moved it earlier, export omits the incompatible start
date and reports that omission instead of changing the user's current dates.
Task start/due properties remain civil-midnight dates; CommonStart/CommonEnd
contain their UTC equivalents using the host timezone.

Steps appear as a checklist in the task note. Versioned
`FerroUI.ToDo.Task` named metadata additionally preserves structured steps,
My Day, rich notes, local recurrence settings, and linked resources for restoration by
this app. If an Outlook user edits the exported note, reimport preserves that
edited note instead of replacing it with the old private-metadata note.

Supported schedules are written as native `PidLidTaskRecurrence` rules with
`PidLidTaskFRecurring` and `PidLidTaskDeadOccurrence`. Numbered series retain the
remaining count; completed historical occurrences cannot generate another task.
Version 3 app metadata preserves richer range/zone/occurrence information only
while Outlook's native projection remains unchanged. Outlook repeat edits and
explicit repeat removal take precedence. [Scheduling](scheduling.md) explains
the distinction between Graph and Outlook multi-weekday ordinal semantics.

Unrepresentable schedules remain in app metadata with an explicit export
warning. The exporter also reports steps rendered into notes. Unsupported
mailbox/RTF/exception forms are not all round-tripped by this exporter. Group names form
folder paths, while presentation details such as themes and category colors
remain application/cloud metadata.

The field profile was checked against Microsoft's
[To Do-to-PST mapping](https://learn.microsoft.com/en-us/graph/todo-concept-overview).
The named `BodyLastModifiedTime` identity uses property set
`{6f305800-7b8b-41af-b73b-84de53a69d3d}`. Existing typed values are retained when
the plain-text note is unchanged; new values with an unspecified
producer-dependent type are not fabricated. Original PST task identifiers stay
in retained source properties while the new PST receives independent
store/node identities. The `hasAttachments` property is derived from the
exported actual file list.

Every referenced file must be available with its recorded size. A missing
cloud file must first be materialized through the connected account; no PST
download is offered with silently omitted contents. Both import and export
detect document edits during asynchronous work and stop rather than applying
an outdated snapshot.

The MIT writer is adapted from
[`johnmcmillan/pst-writer`](https://github.com/johnmcmillan/pst-writer) at commit
`72ac7a5142a1f1f493ccd5d1e91510b7a8593987`. Its pinned source, task extensions,
and notices are in `crates/todo-browser/wwwroot/vendor/pst-writer/`.

## Resource limits and isolation

| Limit | Value |
| --- | --- |
| Input or output PST size | 256 MiB |
| One extracted file | 25 MiB |
| Total extracted file contents | 128 MiB |
| Imported document metadata | 32 MiB |
| Traversed messages | 100,000 |
| Traversed folders | 10,000 |
| Folder depth | 64 |
| Browser parser linear memory | 512 MiB |
| Browser parsing deadline | 60 seconds |

The file-size ceiling is not a guarantee that every file of that size fits the
memory budget. PST structures, extracted file bytes, source properties, and
the serialized restore report all consume memory. The owned-input API avoids
a second complete PST copy inside WASM. Files that exceed processing or memory
limits fail with guidance to export smaller task folders.

Browser parsing loads a separate small WASM module in a disposable worker;
the normal FerroUI/Skia application download does not contain the PST parser.
The parser crate's build script sets the WASM linear-memory maximum. A WASM
trap or allocation error becomes a readable import failure. Timeout, explicit
cancellation, worker errors, and completion all terminate the worker.

The upstream reader can allocate from some internal property counts before
our projection limits run. Therefore the crate is not advertised as a complete
in-process sandbox for arbitrary files. Native hosts run it in an isolated
parser subprocess on Linux, Windows, and macOS with a 60-second deadline,
bounded input/output, and a 1.5 GiB cumulative Rust allocation budget enabled
before reading input. The budget is never refunded on deallocation, so it is a
conservative allocation ceiling rather than a resident-memory measurement.
Linux additionally checks `RLIMIT_AS`; Windows checks a process-memory Job
Object with kill-on-close. The ordinary UI leaves the allocator budget disabled.
Browser hosts use the worker and capped WASM memory. See
[native service isolation](NATIVE-MICROSOFT.md) for the exact host controls.
Application-level checks additionally
bound I/O amplification, reject out-of-file offsets and truncated declared
EOF, enforce folder/message identity uniqueness, and validate named maps and
the resulting document.

## Restore semantics

Parsing completes before attachment storage or document mutation. The prepared
archive then uses the existing portable restore pipeline: validate the
document and all checksums, allocate fresh local attachment keys, stage all
file bytes, and commit once. Failed staging is rolled back. Host coordination
disconnects the previous Microsoft sync relationship and clears its mappings;
restored data is an independent local copy. It is never automatically
published into the previous Microsoft account. Import warnings remain visible
after the restore status is updated.

## Verification and remaining acceptance work

`crates/todo-interop/tests/fixtures/tasks.pst` is deterministic synthetic data.
The JavaScript writer's generated bytes are compared with that committed
fixture. Microsoft's independent Rust reader then checks its Unicode text,
store-specific named properties, a standard Outlook task without app metadata,
completion/reminder dates, categories, private step/repeat metadata, empty task
folder, skipped mail record, and two real files. One file spans multiple PST
blocks to exercise the data-tree path. Tests also cover truncated data,
malformed B-tree/map counts, restore races, worker termination, and missing
exported file contents.

`fidelity.pst` separately verifies source-preserving in-progress, waiting, and
deferred states, low importance, percent complete, start dates, precise source
timestamps, the To Do body-modification named property, and local completion/
importance edits taking precedence. JavaScript tests additionally exercise
start/due UTC companion values across a daylight-saving offset.

`recurrence.pst` verifies standard native recurring tasks, finite counts,
progressed series, completed historical occurrences and external Outlook edits.
`rich-notes.pst` verifies native HTML, safe formatting, plain-body precedence and
edited Outlook notes taking precedence over app metadata.

The emitted WASM is built and exercised against the same PST fixtures. Opening
the result in a desktop Outlook installation remains manual acceptance work;
independent parsing is strong format evidence but is not a claim of testing
every Outlook version, ANSI encoding variant, or external PST producer.

## Microsoft references

- [Microsoft PST reference reader](https://github.com/microsoft/outlook-pst-rs)
- [MS-PST file format](https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/)
- [To Do and PST field mappings](https://learn.microsoft.com/en-us/graph/todo-concept-overview)
- [Task status](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxotask/839a9304-3445-4f52-a879-833817e1dd5a)
- [Task due dates](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxotask/19974e79-5f5b-4ab9-9584-1c555c7718d9)
- [Task start dates](https://learn.microsoft.com/en-us/office/client-developer/outlook/mapi/pidlidtaskstartdate-canonical-property)
- [CommonStart UTC semantics](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxotask/98899e9a-2384-4f63-97cc-fb36d7eacd1e)
- [Task completion dates](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxotask/368f2f64-131b-414e-a23c-c7a18cbd9a23)
- [Reminder properties](https://learn.microsoft.com/en-us/openspecs/exchange_server_protocols/ms-oxormdr/81bb1d6b-9416-4a1e-b8d9-2feedc237bfd)
