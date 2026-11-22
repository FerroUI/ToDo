# FerroUI To Do

A Rust task manager built with **FerroUI**, using Microsoft To Do's Windows layout and workflows as its reference. The interface uses FerroUI's declarative Rust builders, templated controls, WinUI 3 theme, ReactiveUI view models, and DynamicData collections. The browser runs the same Rust UI as the desktop host.

**Live application:** [ferroui.com/ToDo](https://ferroui.com/ToDo/) · [Demo with sample tasks](https://ferroui.com/ToDo/?demo=1). The [Build and publish workflow](https://github.com/FerroUI/ToDo/actions/workflows/pages.yml) publishes successful builds from `main`.

The application includes the main personal task workflows and optional Microsoft Graph synchronization. It does **not** yet establish 100% visual or feature parity with Microsoft's application. See the [feature matrix](docs/features.md) for implemented behavior, platform differences, and remaining work.

## Features

- My Day, Important, Planned, All, Completed, custom lists, and collapsible list groups.
- Task details with steps, step-to-task promotion, notes, importance, due dates, reminders, and repeat schedules.
- English date/time recognition in new task titles, a live schedule preview, and configurable title retention.
- Search across task names, notes, steps, and tags; suggestions; sorting; drag tasks to reorder or move between lists; task and list duplication.
- Multiple selection with modifier-click and ranges, select-all, and bulk completion, move, and deletion.
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
cargo fmt -p todo-domain -p todo-state -p todo-controls -p todo-ui -p todo-browser -p todo-desktop --check
cargo test --locked -p todo-domain -p todo-state
node --test scripts/*.test.mjs
node scripts/ui-smoke.mjs target/browser/todo-browser
```

The Rust suites cover domain behavior, conservative date/time recognition, atomic scheduled-task creation, state transitions, bulk-selection transactions, and callback reentrancy. The JavaScript suites exercise storage conflicts and retries, plus Graph creation, updates, imports, deletion, conflicts, interrupted requests, and account isolation with a simulated service. The browser gate uses the built app and Chromium to check task/list creation, keyboard activation and modal focus restoration, retained title editing, step promotion, and saving a focused note on reload. It also captures desktop, detail, dark-theme, and narrow layouts. Other browser/device combinations need separate validation.

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

`FerroUI/` is a real Git submodule pinned to [`c5a3aadc230c6b830337525eeab79652b12f9cf0`](https://github.com/wieslawsoltes/FerroUI/commit/c5a3aadc230c6b830337525eeab79652b12f9cf0), the latest upstream `main` revision selected for this build. All framework crates resolve through paths into that checkout.

To recover a missing checkout:

```bash
git submodule update --init --recursive
```

To move to the latest upstream `main`, update the gitlink and synchronize the generated workspace dependencies:

```bash
git submodule update --remote FerroUI
python3 FerroUI/scripts/apps/sync-workspace.py
cargo update --workspace
bash scripts/build-browser.sh
```

Commit the updated submodule pointer, manifest, and lockfile together. CI checks out the recorded revision recursively, so each deployed build remains reproducible.

## Deployment

The repository uses one workflow with a cached browser build and a Pages deployment job. A short post-deployment HTTP check verifies the page, JavaScript, WASM, and narrow-layout preview over HTTPS. It runs on pushes to `main` and manual dispatch. The toolchain comes from FerroUI's setup script. An explicit compiler cache retains its submodule path dependencies, keyed by the pinned framework revision, toolchain, and build configuration; successful compilation is cached before the browser gate. There is no platform matrix or release-packaging pipeline. Configure repository Pages to use **GitHub Actions**; the workflow uploads the generated site and publishes it using GitHub's Pages actions.

## License

Application code is [MIT licensed](LICENSE). Third-party notices and the independent implementation's relationship to Microsoft To Do are documented in [NOTICE.md](NOTICE.md). FerroUI and its bundled dependencies retain their own licenses and notices.
