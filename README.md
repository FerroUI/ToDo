# FerroUI To Do

A Rust task manager built with **FerroUI**, using Microsoft To Do's Windows layout and workflows as its reference. The interface uses FerroUI's declarative Rust builders, templated controls, WinUI 3 theme, ReactiveUI view models, and DynamicData collections. The browser runs the same Rust UI as the desktop host.

**GitHub Pages deployment target:** [ferroui.github.io/ToDo](https://ferroui.github.io/ToDo/). The [Build and publish workflow](https://github.com/FerroUI/ToDo/actions/workflows/pages.yml) publishes successful builds from `main`.

The application includes the main personal task workflows and optional Microsoft Graph synchronization. It does **not** yet establish 100% visual or feature parity with Microsoft's application. See the [feature matrix](docs/features.md) for implemented behavior, platform differences, and remaining work.

## Features

- My Day, Important, Planned, All, Completed, custom lists, and collapsible list groups.
- Task details with steps, notes, importance, due dates, reminders, and repeat schedules.
- Search across task names, notes, steps, and tags; suggestions; sorting; manual move commands; task and list duplication.
- Calendar-aware daily, weekday, weekly, monthly, yearly, and custom interval recurrence.
- Undo/redo, JSON backup and restore, print view, and browser file attachments.
- Light, dark, and system themes; per-list colors; original vector icons and My Day landscape.
- Responsive desktop and narrow layouts, virtualized task rows, keyboard shortcuts, and FerroUI accessibility support.
- Browser storage, installable app metadata, and offline resources cached after a successful load.
- Optional Microsoft account connection and explicit **Sync now** for supported list/task fields and checklist items.

A normal launch starts with an empty Tasks list. Add `?demo=1` to the browser URL, or pass `--demo` to the desktop executable, to inspect sample tasks without modifying the normal task document.

## Build the browser app

Install Git, Rust through rustup, Node.js 24 with npm, Python 3, and the native build prerequisites described by [FerroUI](FerroUI/README.md). The browser setup script supports Linux and macOS and installs its pinned toolchain into the selected tools directory.

```bash
git clone --recurse-submodules https://github.com/FerroUI/ToDo.git
cd ToDo
bash FerroUI/scripts/browser/setup.sh "$PWD/.tools"
source .tools/env.sh
bash scripts/build-browser.sh
python3 -m http.server 8080 --directory target/browser/todo-browser
```

Open [localhost:8080](http://localhost:8080/). Use [localhost:8080/?demo=1](http://localhost:8080/?demo=1) for the sample document.

The output directory is `target/browser/todo-browser`. The wrapper delegates compilation to FerroUI, then generates the offline resource manifest and `.nojekyll`. It accepts FerroUI's build options, for example:

```bash
bash scripts/build-browser.sh --debug
bash scripts/build-browser.sh --out target/site
```

The target is **`wasm32-unknown-emscripten`**. The pinned browser toolchain is Rust **1.99.0**, Emscripten **6.0.10**, and wasm-bindgen **0.2.129**. Use the setup script rather than substituting the `wasm32-unknown-unknown` target or an independently selected wasm-bindgen version.

## Run the desktop host

After installing FerroUI's platform prerequisites:

```bash
cargo run --locked -p todo-desktop --profile fast-release
cargo run --locked -p todo-desktop --profile fast-release -- --demo
```

The desktop host shares the UI and model with the browser. It saves `tasks.json` in the platform's application-data directory under `FerroUI/ToDo`; `TODO_DATA_DIR` overrides that directory. Browser attachments, Microsoft sign-in/sync, system notifications, and completion audio are not all implemented by the desktop host; see [platform coverage](docs/features.md#platform-coverage).

## Microsoft account connection

The local application works without an account. Optional synchronization requires **your own Microsoft Entra application registration configured as a single-page application**, its Application (client) ID, an exact redirect URI for the deployment, and consent for `Tasks.ReadWrite` and `User.Read`.

Follow the [Microsoft account setup guide](docs/MICROSOFT.md), enter the client ID in Settings, connect the Microsoft account, and choose **Sync now**. Authentication uses the bundled Microsoft Authentication Library; tokens stay in session storage and are excluded from task documents and backups. There is no client secret or application-owned backend.

The integration has automated synchronization tests with a simulated Graph service. A live account sign-in and round-trip sync have not been verified as part of this implementation. Sharing invitations, Planner integration, Microsoft attachment synchronization, and complete Microsoft service parity remain outside the implemented surface. [Features and limits](docs/features.md#microsoft-graph-synchronization) describes the boundary.

## Check changes

```bash
cargo fmt --all --check
cargo test --locked -p todo-domain -p todo-state
node --test scripts/graph-sync.test.mjs
```

Initial validation includes 25 passing domain and state tests. The Graph suite tests creation, updates, imports, deletion, conflicts, interrupted requests, and account isolation with a simulated service. Browser rendering and interaction still need validation on the intended browser/device combinations.

## Architecture

| Crate | Responsibility |
| --- | --- |
| `todo-domain` | Serializable entities, commands, queries, calendar rules, and backup validation |
| `todo-state` | ReactiveUI commands, DynamicData updates, navigation, undo/redo, and host requests |
| `todo-controls` | WinUI theme integration, resources, control templates, and original vector icons |
| `todo-ui` | Declarative Rust views, virtualized row projection, detail panels, and dialogs |
| `todo-browser` | FerroUI browser startup, persistence, offline assets, files, notifications, and Graph bridge |
| `todo-desktop` | Native startup, application-data persistence, file dialogs, and platform services |

Read [Architecture](docs/architecture.md) for state flow, performance decisions, and extension points.

## FerroUI submodule

`FerroUI/` is a real Git submodule pinned to [`ef88c554005a0c52aeeb0083a6d577cd62cffbfd`](https://github.com/wieslawsoltes/FerroUI/commit/ef88c554005a0c52aeeb0083a6d577cd62cffbfd), the upstream `main` revision selected when this application was created. All framework crates resolve through paths into that checkout.

To recover a missing checkout:

```bash
git submodule update --init --recursive
```

When deliberately updating FerroUI, update its submodule commit, run `python3 FerroUI/scripts/apps/sync-workspace.py`, update the lockfile, and verify the browser build. The generated workspace dependencies and profiles should remain aligned with the selected framework revision.

## Deployment

The repository uses one workflow with a cached browser build and a Pages deployment job. It runs on pushes to `main` and manual dispatch. There is no platform matrix, benchmark upload, or release-packaging pipeline. Configure repository Pages to use **GitHub Actions**; the workflow uploads the generated site and publishes it using GitHub's Pages actions.

## License

Application code is [MIT licensed](LICENSE). Third-party notices and the independent implementation's relationship to Microsoft To Do are documented in [NOTICE.md](NOTICE.md). FerroUI and its bundled dependencies retain their own licenses and notices.
