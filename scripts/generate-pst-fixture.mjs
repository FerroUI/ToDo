import { writeFile } from "node:fs/promises";
import { fixturePst, fidelityPst } from "./tests/pst-fixture.mjs";
// Fixed local timezone makes UTC completion/reminder timestamps reproducible.
process.env.TZ="UTC";
await writeFile(new URL("../crates/todo-interop/tests/fixtures/tasks.pst",import.meta.url),new Uint8Array(await fixturePst().arrayBuffer()));
await writeFile(new URL("../crates/todo-interop/tests/fixtures/fidelity.pst",import.meta.url),new Uint8Array(await fidelityPst().arrayBuffer()));
