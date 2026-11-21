# Architecture

## Boundaries

The application is a six-crate Rust workspace. Domain behavior does not depend on the renderer or host. Shared views depend on portable state and controls; browser and desktop crates provide the platform boundary.

| Layer | Crate | Owns |
| --- | --- | --- |
| Domain | `todo-domain` | Tasks, lists, groups, settings, dates, recurrence, commands, queries, JSON validation |
| Application | `todo-state` | ReactiveUI command dispatch, DynamicData cache, selection, navigation, history, persistence orchestration |
| Presentation primitives | `todo-controls` | WinUI base theme, light/dark resources, app-specific control themes, original vector geometry |
| Views | `todo-ui` | Rust control builders, sidebar, list, task details, calendar/repeat dialogs, settings |
| Browser host | `todo-browser` | Emscripten startup, JavaScript host requests, storage, Graph integration, offline resources |
| Desktop host | `todo-desktop` | Native startup, filesystem persistence, platform file pickers, launcher |

The `FerroUI/` submodule is pinned by Git. Application dependencies resolve to that checkout, with the framework's dependency versions and build profiles synchronized into the root workspace manifest. This keeps the framework source and application build reproducible without waiting for a published crate version.

## State and command flow

A view sends a typed `Action` to `AppViewModel::dispatch`. ReactiveUI executes the command on the application thread. The domain validates and applies it, returning `Changes` with affected entity identifiers. The view model records an undo entry, publishes only affected tasks through `SourceCache<Task, Id>`, saves the document, and raises its revision property. Host operations use a separate typed `HostRequest` boundary.

`AppState` is the authoritative document. The DynamicData cache is its observable task projection. Navigation, selected rows, modal panels, sidebar visibility, viewport width, cloud status, and command execution state are transient. Settings and list presentation choices are persisted.

Reentrant commands are queued until the current command completes. Document and host-service borrows are released before subscriber or platform callbacks run. A task added to a smart view uses one `AddTaskInView` action, so creation and membership are atomic without guessing the next identifier or cloning the entire document. General batches retain transactional rollback semantics.

## Views and templates

Views are constructed with FerroUI's declarative Rust builders and reactive component lifecycle. They create real FerroUI controls, use the framework's property/layout/input/accessibility systems, and render through its browser or desktop backend. HTML provides the browser mount, loading surface, installation metadata, and host integrations; the task interface is rendered by Rust controls.

`todo-controls` starts from `ferroui-themes-winui`. Explicit theme roots retain the controls required by the app. App-specific resources and `ControlTheme` templates provide flat task actions and list item presentation while retaining the `ListBox` scroll host and virtualizing panel. Template bindings connect presenter properties to templated control properties. Light and dark palettes share the same control composition.

The task list has a stable observable `FerroList` and one recyclable data template for its lifetime. Rows carry only render-relevant data: identity, title, completion, importance, metadata, selection, and appearance. Large notes and attachment metadata stay in the document. Ordinary edits patch matching rows; large reorder/filter changes replace the projection rather than performing repeated quadratic searches and moves. Sidebar counts are calculated without sorting task lists. Search input is debounced on desktop.

At narrow widths, the sidebar, list, and details become separate full-width panes. Above the narrow breakpoint, the default composition uses a 280-pixel sidebar and a 340-pixel details pane around the flexible task list.

Task selection has a separate mode, range anchor, and keyboard cursor. Modifier clicks and keyboard ranges use the same visible ordering as the row projection, including date groups and collapsed completed rows. Navigation resets the selection and anchor. Bulk completion, movement, and deletion dispatch one domain batch, so one undo restores the entire operation. Selection filters stale entity identifiers before executing commands.

Keyboard navigation reveals and focuses the corresponding virtualized container. The UI is split into small modules for row actions, list keyboard handling, selection controls, sidebar sections, and shell input. A shared edit-flush helper collects pending title, note, and step changes before host close/export paths can unmount the controls, then dispatches after releasing control/document reads.

## History and persistence

Undo entries retain affected task entities and, when needed, list/group topology or settings. They are capped at 30 entries and approximately 8 MB, while retaining at least the newest entry. Restoring an entry creates its inverse for redo. Identifiers are not recycled after undo.

The browser document lives in `localStorage`; attachment blobs live in IndexedDB. Authentication tokens and account-specific Graph mapping/checkpoint metadata are separate from the task document. The desktop saves through a temporary file followed by rename in its application-data directory.

Each browser tab compares the stored document with its last successfully loaded or saved value before writing. A stale tab keeps its edits in memory and requests a backup and reload instead of replacing the newer saved document. A failed write never advances that baseline. This detects stale-tab writes; `localStorage` does not provide an atomic transaction across simultaneous writers.

JSON restoration parses into an isolated document and validates schema version, entity identifiers, references, date-related values, and attachment metadata before changing live state. Imports and cloud replacements must save successfully before replacing the current document. An ordinary edit remains in memory if persistence fails, and the UI reports the failure. A subsequent successful save clears that error. Unreadable startup data is preserved as a recovery copy where the host can do so.

Backups contain task data and attachment metadata, **not attachment blob contents**. Files do not automatically transfer to another browser or device through a JSON backup.

## Dates and reminders

The host supplies the local civil date; domain logic never assumes UTC midnight is the user's day boundary. My Day includes explicit current-day membership and tasks due today, with a per-day exclusion for explicit removal.

Recurrence keeps an original anchor so monthly schedules preserve their intended day after short months, and yearly schedules preserve leap-day intent. Completion retains the completed occurrence and creates the next future occurrence, resetting steps and per-day membership. Reopening and completing the original again does not create another successor.

The browser and desktop tick reminders while the app is open. Delivery is recorded before calling host notification code, updated in the observable cache, and saved once per tick. Undo preserves delivery markers so an unrelated edit cannot cause the same reminder to fire again. There is no server push scheduler for a closed application.

## Optional Microsoft synchronization

The browser uses bundled MSAL for OAuth authorization-code flow with PKCE and session-scoped token caching. Graph requests use delegated user permissions and the public Graph v1.0 endpoints. The user supplies an Entra SPA application ID; no client secret or service backend is embedded.

The sync adapter maps local and remote entity identities, tracks field baselines, follows paginated collections, and checkpoints remote mutations. It merges non-conflicting field edits and stops on conflicting changes rather than silently replacing one side. Conditional writes use ETags where available. Linked-resource identifiers help recover uncertain task creation without duplicating the task.

Web Locks prevent two tabs from running the sync operation simultaneously. The Rust bridge compares the current exported document with the snapshot used by the sync before accepting a replacement. If local edits happened during network requests, the replacement is rejected and those edits remain. Remote changes may already have been applied; checkpoints support a subsequent retry. This is explicit synchronization, not a transactional operation spanning Microsoft and local storage.

See [feature coverage](features.md#microsoft-graph-synchronization) for supported fields and unsupported Microsoft service behavior. Automated Graph tests use a simulated service; live account validation remains separate.

## Build and verification

The browser uses FerroUI's pinned Emscripten toolchain and size-oriented browser profile. The build wrapper generates content-versioned offline assets after compilation. The app uses the single-threaded browser path by default, so deployment does not require a threaded shared-memory configuration.

Tests cover domain calendar rules and relationships, smart-list commands, undo/redo, affected-entity cache updates, callback reentrancy, persistence failures, reminder deduplication, imports, and cloud replacement semantics. Selection tests verify that bulk operations remain atomic across undo, navigation resets anchors, and select-all matches visible completed-group behavior. Graph tests exercise the adapter against controlled service responses. The Pages workflow keeps one build path and a single rendering smoke check rather than a platform matrix.

No application-specific frame-time or large-dataset performance budget has been established yet. Virtualization, retained row sources, cached icon geometry, theme trimming, and targeted state updates are implementation choices; they should not be read as measured performance guarantees.
