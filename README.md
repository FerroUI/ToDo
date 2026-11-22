# FerroUI To Do

A Rust task manager built with **FerroUI**, using Microsoft To Do's Windows layout and workflows as its reference. The interface uses FerroUI's declarative Rust builders, templated controls, WinUI 3 theme, ReactiveUI view models, and DynamicData collections. The browser runs the same Rust UI as the desktop host.

**Live application:** [ferroui.com/ToDo](https://ferroui.com/ToDo/) · [Demo with sample tasks](https://ferroui.com/ToDo/?demo=1). The [Build and publish workflow](https://github.com/FerroUI/ToDo/actions/workflows/pages.yml) publishes successful builds from `main`.

The application includes the main personal task workflows and optional Microsoft Graph synchronization. It does **not** yet establish 100% visual or feature parity with Microsoft's application. See the [feature matrix](docs/features.md) for implemented behavior, platform differences, and remaining work.

## Features

- My Day, Important, Planned, All, Completed, custom lists, and collapsible list groups.
- Task details with steps, step-to-task promotion, notes, importance, due dates, reminders, and repeat schedules.
- English date/time recognition in new task titles, a live schedule preview, and configurable title retention.
- Search across task names, notes, steps, and tags; suggestions; sorting; drag tasks to reorder or move between lists; task and list duplication.
- Multiple selection with modifier-click and ranges, native touch hold, select-all, and bulk completion, move, and deletion.
- Touch swipe actions for Delete, My Day and Move; hold-and-drag task/list/group organization, scroll arbitration, and cancellation.
- Browser/keyboard/edge Back, swipe-to-dismiss navigation, and pull-to-refresh without losing drafts or reloading the app.
- Native anchored task/list/group/scheduling menus, selection-aware task commands, and inline list/group naming.
- Mixed list/group navigation ordering, list-to-group dragging, keyboard tree navigation, and leading-emoji list icons.
- Visible sort direction with reverse/reset controls, search cancellation, native Calendar selection, and WinUI Settings switches.
- Calendar-aware daily, weekday, weekly, monthly, yearly, and custom interval recurrence.
- Undo/redo, JSON backup and restore, portable backups with attachment bytes and checksums, Microsoft Graph data archives, Outlook task PST import/export, and print view.
- Light, dark, and system themes; per-list colors; original vector icons and My Day landscape.
- Adaptive one-, two-, and three-pane layouts, retained drafts and keyboard focus, virtualized task rows, and FerroUI accessibility support.
- Browser storage, installable app metadata, and offline resources cached after a successful load.
- Optional Microsoft sign-in with MSAL and PKCE, automatic foreground synchronization, explicit **Sync now**, cloud file attachments, categories, and linked source actions.
- Optional **Assigned to me** integration for work-account Planner tasks, including title, completion, due date, notes, and checklist edits.
- Optional Outlook master-category catalogue with color-aware selection and separate `MailboxSettings.Read` consent.
- Optional OneDrive app-folder synchronization of My Day, groups, ordering, and appearance between installations using the same Microsoft application registration.
- Fixed global, GCC High, and DoD identity/Graph configurations; unsupported China-operated To Do services are reported explicitly.
- Desktop Microsoft services through an isolated Node.js worker, system-browser PKCE login, local attachment storage, and optional tray operation when the platform supports it.

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

The desktop host shares the UI and model with the browser. It saves `tasks.json` in the platform's application-data directory under `FerroUI/ToDo`; `TODO_DATA_DIR` overrides that directory. Its Microsoft, attachment, and portable/PST archive services run in an isolated **Node.js 22 or later** worker; `TODO_NODE_BINARY` selects a particular Node executable. The worker uses vendored dependencies and requires no runtime npm install. Local task editing continues without the worker.

Native authentication opens the system browser and uses a registered **Mobile and desktop applications** loopback redirect, normally `http://localhost/`. An encrypted MSAL cache can resume the explicitly selected account after restart, with its encryption key held in the OS credential store. An unavailable credential store falls back visibly to a session-only connection. Disconnect forgets the saved session. An optional **Keep running in the background** setting keeps sync and reminders active after closing the window only when a usable system tray is available; Quit still stops the process. Native PST import runs in an isolated child on Linux, Windows, and macOS with a deadline and a 1.5 GiB cumulative Rust allocation budget; Linux/Windows also enforce OS memory limits. See the [native service guide](docs/NATIVE-MICROSOFT.md) for setup, platform gates, and verification limits.

## Microsoft account connection

The local application works without an account. Optional synchronization requires **your own Microsoft Entra application registration**, its public Application (client) ID, correctly registered browser/native redirect URIs, and delegated consent for `Tasks.ReadWrite` and `User.Read`. The owner-run registration helper can produce a reviewable registration plan and apply it through an already authenticated Azure CLI session; it does not log in, create secrets, or grant consent automatically.

Follow the [Microsoft account setup guide](docs/MICROSOFT.md). Configure the public client ID at deployment time, or enter it in Settings for a local setup. Connect the account to enable automatic synchronization; **Sync now** and pull-to-refresh also synchronize on demand. Work accounts can enable **Settings → Connected apps → Planner**. Outlook category colors and OneDrive device settings each request their own optional permission only when enabled; declining either preserves the existing task connection. Browser tokens use session storage; native persistence uses an OS-protected encrypted cache. Credentials are excluded from task documents and backups. There is no client secret or application-owned server.

The Pages build accepts repository variables `MICROSOFT_CLIENT_ID`, `MICROSOFT_TENANT`, `MICROSOFT_REDIRECT_URI`, and `MICROSOFT_CLOUD`; the corresponding local build variables have a `TODO_` prefix. The canonical production redirect is `https://ferroui.com/ToDo/`. These are public settings, not secrets. Browser SPA and native public-client redirects are separate entries on the same registration. Native configuration uses `TODO_MICROSOFT_NATIVE_REDIRECT_URI` for its loopback address.

The integration has automated tests for OAuth, cloud/scope isolation, Graph transport, delta synchronization, files, interrupted writes, Planner, category caching, OneDrive replica merging, PST/portable archives, and host orchestration. A live Microsoft login/round-trip and acceptance of the exported PST in desktop Outlook have not been verified. Invitation administration, shared-list assignee editing, Microsoft's private My Day/group metadata, rich native Outlook recurrence export, and closed-process push remain outside the implemented surface. OneDrive settings sync is an independent app format and does not synchronize Microsoft's private client state. See the [integration review](docs/integration-review.md), [PST format guide](docs/OUTLOOK-PST.md), and [feature matrix](docs/features.md#microsoft-graph-synchronization) for exact boundaries.

## Check changes

```bash
cargo fmt -p todo-domain -p todo-state -p todo-controls -p todo-ui -p todo-browser -p todo-desktop -p todo-interop -p todo-interop-wasm -p todo-credentials --check
cargo test --locked -p todo-domain -p todo-state -p todo-interop -p todo-credentials
node --test scripts/*.test.mjs
node scripts/ui-smoke.mjs target/browser/todo-browser
```

The Rust suites cover domain behavior, conservative date/time recognition, atomic scheduled-task creation, state transitions, bulk-selection transactions, inline naming, sidebar ordering, adaptive pane budgets, and callback reentrancy. The JavaScript suites exercise storage conflicts, OAuth redirects and token refresh, incremental Graph reads, files and resumable uploads, guarded moves and deletion, interrupted requests, account isolation, Planner edits, complete-file archive restoration, and the shared account/restore/sync queue with simulated services. One browser gate uses the built app and Chromium to check real editing, menus, selection, focus restoration, drag/drop, draft retention, and persistence on reload. Real touch contacts exercise swipe actions, long press, scrolling, cancellation, navigation, and refresh; browser Back also verifies focused-editor persistence. It verifies widths of 320, 390, 760, 800, 980, and 1024 pixels and captures desktop, intermediate, narrow, menu, and dark-theme views. Other browser/device combinations need separate validation.

## Architecture

| Crate | Responsibility |
| --- | --- |
| `todo-domain` | Serializable entities, commands, queries, calendar rules, and backup validation |
| `todo-state` | ReactiveUI commands, DynamicData updates, navigation, undo/redo, and host requests |
| `todo-controls` | WinUI theme integration, resources, control templates, and original vector icons |
| `todo-ui` | Declarative Rust views, virtualized row projection, detail panels, and dialogs |
| `todo-browser` | FerroUI browser startup, persistence, offline assets, files, notifications, and Graph bridge |
| `todo-desktop` | Native startup, persistence, file dialogs, tray/notifications, and isolated service-worker bridge |
| `todo-interop` | Bounded Outlook PST parsing and task/property/attachment projection |
| `todo-interop-wasm` | Dedicated browser-worker WASM entry point for the shared PST reader |
| `todo-credentials` | Bounded native credential-store protocol and OS-backed cache-key storage |

Read [Architecture](docs/architecture.md) for state flow, performance decisions, and extension points. [Reference behavior](docs/reference-behavior.md) records the Microsoft documentation used for interaction decisions and distinguishes verified workflows from inferred adaptive thresholds. The [UI, UX, and feature review](docs/ui-ux-review.md) records the full review, this polishing round, and remaining fidelity gaps.

## FerroUI submodule

`FerroUI/` is a real Git submodule pinned by the repository's gitlink. Run `git submodule status FerroUI` to inspect the exact recorded revision. All framework crates resolve through paths into that checkout.

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
