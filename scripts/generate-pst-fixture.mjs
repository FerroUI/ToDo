import { writeFile } from "node:fs/promises";
import { fixturePst, fidelityPst } from "./tests/pst-fixture.mjs";
import { richNotesPst } from "./tests/rich-notes-pst-fixture.mjs";
import { recurrencePst } from "./tests/recurrence-pst-fixture.mjs";
// Fixed local timezone makes UTC completion/reminder timestamps reproducible.
process.env.TZ="UTC";
await writeFile(new URL("../crates/todo-interop/tests/fixtures/tasks.pst",import.meta.url),new Uint8Array(await fixturePst().arrayBuffer()));
await writeFile(new URL("../crates/todo-interop/tests/fixtures/fidelity.pst",import.meta.url),new Uint8Array(await fidelityPst().arrayBuffer()));
await writeFile(new URL("../crates/todo-interop/tests/fixtures/rich-notes.pst",import.meta.url),new Uint8Array(await richNotesPst().arrayBuffer()));
await writeFile(new URL("../crates/todo-interop/tests/fixtures/recurrence.pst",import.meta.url),new Uint8Array(await recurrencePst().arrayBuffer()));
