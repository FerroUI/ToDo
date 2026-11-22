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

The composer can recognize conservative English scheduling suffixes into a `TaskEntry`. Its live preview and submission use the same parser; `AddTaskEntry` creates the title, schedule, and smart-view membership in one command and one undo step. Recognition can be disabled, and original text is retained unless the user enables removal. Detail editing and direct view-model add calls remain literal.

## Views and templates

Views are constructed with FerroUI's declarative Rust builders and reactive component lifecycle. They create real FerroUI controls, use the framework's property/layout/input/accessibility systems, and render through its browser or desktop backend. HTML provides the browser mount, loading surface, installation metadata, and host integrations; the task interface is rendered by Rust controls.

`todo-controls` starts from `ferroui-themes-winui`. Explicit theme roots retain the controls required by the app. App-specific resources and `ControlTheme` templates provide flat task actions and list item presentation while retaining the `ListBox` scroll host and virtualizing panel. Template bindings connect presenter properties to templated control properties. Light and dark palettes share the same control composition.

The task list has a stable observable `FerroList` and one recyclable data template for its lifetime. Rows carry only render-relevant data: identity, title, completion, importance, metadata, selection, and appearance. Large notes and attachment metadata stay in the document. Ordinary edits patch matching rows; large reorder/filter changes replace the projection rather than performing repeated quadratic searches and moves. Sidebar counts are calculated without sorting task lists. Search input is debounced on desktop.

`todo-state::PaneLayout` resolves pane topology from width, the persisted-in-session sidebar preference, the temporary drawer state, and whether a right pane is open. A three-pane layout requires 280px navigation + 360px minimum list + 340px details. At 760–979px with details/settings/suggestions open, navigation becomes an overlay so the center remains useful. Below 760px the list and right pane occupy the same full-width slot one at a time. Without a right pane, navigation can be inline from 760px. The browser deducts the framework-reported horizontal safe-area padding before choosing the pane topology. A retained native InputPaneAwareDecorator consumes reported keyboard occlusion inside the already padded host, avoiding duplicate safe-area compensation. Layout invalidation occurs only when this topology changes; Grid handles ordinary pixel resizing. These thresholds are documented implementation decisions, not extracted Microsoft constants.

Inline naming is a view-model transaction (`RenameTarget`, `RenameSession`) separate from the domain document. Creation persists the untitled entity immediately; typing changes only its transient draft. A queued command's completion receives its own `Changes`, so reentrant creation opens the correct editor without predicting an identifier. Enter or blur takes the transaction before dispatching one rename, preventing LostFocus reentry from generating another command. Escape, navigation, removal, and stale target callbacks cannot commit a canceled edit. Per-task new-step drafts are likewise transient and survive right-pane replacement.

Native menus use the framework's ContextMenu/MenuItem/Separator templates. Each attached context menu holds weak anchor/menu references and builds one owned item tree at opening. A recycled task row resolves its current tagged identity and the current selection at that point, so an unchanged selected row cannot retain an obsolete selection snapshot. Sidebar commands use explicit list/group IDs. Commands close the popup before mutations; selected movement clears the selection only after success. Keyboard context requests open on key release, preventing the native Apps-key release from immediately closing a just-opened popup.

Task selection has a separate mode, range anchor, and keyboard cursor. Modifier clicks and keyboard ranges use the same visible ordering as the row projection, including date groups and collapsed completed rows. Navigation resets the selection and anchor. Bulk completion, movement, and deletion dispatch one domain batch, so one undo restores the entire operation. Selection filters stale entity identifiers before executing commands.

Keyboard navigation reveals and focuses the corresponding virtualized container. The UI is split into small modules for row actions, list keyboard handling, selection controls, sidebar sections, menus, inline naming, and shell input. A shared edit-flush helper collects pending account, title, note, step, and inline-name changes before host close/export paths can unmount the controls, then dispatches after releasing control/document reads.

Shell shortcuts attach once to the current `TopLevel` and detach with the root. They remain reachable when recycling removes the focused child, while respecting keys already handled by controls. Route ownership and command handling live in separate small modules.

Focused row buttons own their Enter/Space activation; container shortcuts do not intercept their key-down and leave a mismatched key-up. Pane, modal, and menu focus restoration uses weak references and deferred work. Pane transitions carry a generation token, and drawer lookup stays within its navigation surface so it cannot focus an obscured duplicate trigger. Opening a right pane focuses its Close button without opening a touch keyboard; returning from Settings to a task preserves the prior control when possible. Native menus consume application shortcuts while they own input. Restore paths yield to replacement surfaces and valid focus, then use the origin, current task container, relevant editor, or programmatically focusable app root.

Task dragging uses FerroUI's in-process drag transport with a private typed payload and weak originating-view-model identity. Recycled row and sidebar controls resolve identities from their current tags. A mouse movement threshold separates clicks from drags. Hover adds feedback without editing the document; a validated drop dispatches one reorder or move command. Moving between lists appends to the destination order in the domain action, without a full-document batch clone. Synchronous cancellation invalidates the payload before queued pointer events can produce a late drop. Detail surfaces are keyed by task identity so editor lifetimes do not cross between tasks.

## Touch, Back, and refresh

Touch uses FerroUI gesture recognizers and native pointer capture. A pure `TouchTrack` in `todo-state` decides tap/hold, horizontal, vertical, or canceled intent after a direction-lock threshold. Native scrolling retains vertical gestures until an eligible list-top pull is claimed. Task swipes translate the existing templated card and reveal native action buttons; no persistent action occurs until the user activates one. Holding enters selection, and continuing to move starts manual drag using the same validated planners as mouse input.

The framework's native `SwipeControl`, `RefreshContainer`, and `RefreshVisualizer` were reviewed at the pinned revision. Their handled-input, capture, cancellation, and public-provider boundaries currently prevent a direct replacement that preserves these interactions. The [native gesture control integration review](native-gesture-controls.md) records the source evidence and supported migration options.

Custom list/group rows use the same native Holding/capture path. A stationary release opens their existing context menu; moving after the hold begins organization. The typed task/sidebar operation shares before/after/into-group planning with mouse dragging and excludes protected built-in entries.

Native Holding transfers the contact to the mounted shell before task selection notifies the collection: a native Replace can recycle the source row even when its logical task ID is unchanged. The shell retains the held contact without drag feedback or autoscroll until movement crosses the threshold. Its operation stores the task identity and originating view; the source visual is only a weak reference for decoration. Recycling a source row therefore cannot redirect a drop to its replacement identity. Hover/autoscroll and release revalidate current state, and only a qualified release can dispatch the final operation. Escape, an additional contact, cancellation/capture loss, wheel input, route changes, and teardown clear pending state and timers. Additional contacts in a canceled gesture are quarantined until all fingers lift, while the original native pointer still receives its release/cancel for disposal. Handlers, subscriptions, and recognizers are removed with their owning lifecycle.

Browser, edge, and Alt+Left Back use one Rust dismissal path. Native menus close first; transient naming cancels before an edit flush can commit it. Ordinary title, step, note, and account edits flush before their surfaces disappear. Remaining priority is the temporary drawer, panel, details, selection, search, then a valid prior view. View history is transient, bounded to 64 entries, excludes repeated search typing, and skips removed list IDs. Escape only dismisses surfaces and does not traverse view history.

The browser host maintains at most one same-document history guard. It preserves the URL and unrelated history state, does not navigate during startup/reload, and only handles a user Back traversal. Closing a surface through its own UI never triggers asynchronous `history.back()`. Refresh is a single-flight host request: flush normal edits, advance the local civil clock, optionally invoke the existing explicit Microsoft sync adapter, then clear the in-flight flag in a `finally` path. The mounted UI and unsubmitted composer/step drafts remain intact.

## History and persistence

Undo entries retain affected task entities and, when needed, list/group topology or settings. They are capped at 30 entries and approximately 8 MB, while retaining at least the newest entry. Restoring an entry creates its inverse for redo. Identifiers are not recycled after undo. Status Undo is explicitly associated with the just-reported deletion/move and invalidated by a later edit; ordinary errors cannot expose an unrelated history action. Repeated Undo/Redo status text still raises a revision so non-cache surfaces reconcile. The status footer reserves layout space and transfers focus to a retained control before an action removes itself.

The browser document lives in `localStorage`; attachment blobs live in IndexedDB. Authentication tokens and account-specific Graph mapping/checkpoint metadata are separate from the task document. The desktop saves through a temporary file followed by rename in its application-data directory.

Each browser tab compares the stored document with its last successfully loaded or saved value before writing. A stale tab keeps its edits in memory and requests a backup and reload instead of replacing the newer saved document. A failed write never advances that baseline. This detects stale-tab writes; `localStorage` does not provide an atomic transaction across simultaneous writers.

JSON restoration parses into an isolated document and validates schema version, entity identifiers, references, date-related values, and attachment metadata before changing live state. Imports and cloud replacements must save successfully before replacing the current document. An ordinary edit remains in memory if persistence fails, and the UI reports the failure. A subsequent successful save clears that error. Unreadable startup data is preserved as a recovery copy where the host can do so.

Backups contain task data and attachment metadata, **not attachment blob contents**. Files do not automatically transfer to another browser or device through a JSON backup.

## Dates and reminders

The host supplies the local civil date; domain logic never assumes UTC midnight is the user's day boundary. My Day includes explicit current-day membership and tasks due today, with a per-day exclusion for explicit removal.

Recurrence keeps an original anchor so monthly schedules preserve their intended day after short months, and yearly schedules preserve leap-day intent. Completion retains the completed occurrence and creates the next future occurrence, resetting steps and per-day membership. Reopening and completing the original again does not create another successor. Saving the unchanged due date or repeat rule preserves the original anchor and occurrence marker; changing the rule or date deliberately reanchors the schedule.

The browser and desktop tick reminders while the app is open. Delivery is recorded before calling host notification code, updated in the observable cache, and saved once per tick. Undo preserves delivery markers so an unrelated edit cannot cause the same reminder to fire again. Saving an unchanged reminder preserves its delivery marker, including after reload; changing its date or time arms the new schedule. There is no server push scheduler for a closed application.

## Optional Microsoft synchronization

The browser uses bundled MSAL for OAuth authorization-code flow with PKCE and session-scoped token caching. Graph requests use delegated user permissions and the public Graph v1.0 endpoints. The user supplies an Entra SPA application ID; no client secret or service backend is embedded.

The sync adapter maps local and remote entity identities, tracks field baselines, follows paginated collections, and checkpoints remote mutations. It merges non-conflicting field edits and stops on conflicting changes rather than silently replacing one side. Conditional writes use ETags where available. Linked-resource identifiers help recover uncertain task creation without duplicating the task.

Web Locks prevent two tabs from running the sync operation simultaneously. The Rust bridge compares the current exported document with the snapshot used by the sync before accepting a replacement. If local edits happened during network requests, the replacement is rejected and those edits remain. Remote changes may already have been applied; checkpoints support a subsequent retry. This is explicit synchronization, not a transactional operation spanning Microsoft and local storage.

See [feature coverage](features.md#microsoft-graph-synchronization) for supported fields and unsupported Microsoft service behavior. Automated Graph tests use a simulated service; live account validation remains separate.

## Build and verification

The browser uses FerroUI's pinned Emscripten toolchain and size-oriented browser profile. The build wrapper generates content-versioned offline assets after compilation. The app uses the single-threaded browser path by default, so deployment does not require a threaded shared-memory configuration.

CI invokes the framework's canonical setup script and saves the installed tools before later gates. A separate compiler cache retains the submodule path dependencies that a generic workspace cache would prune. Its exact key includes the clean framework gitlink, compiler identity, manifests, lockfile, configuration, and profile environment. CI normalizes tracked framework source and source-directory timestamps to the pinned commit for Cargo freshness checks; generated content remains untouched. Compiler artifacts are saved after compilation and before browser checks, excluding the generated site and QA files. A fallback prefix is constrained to the same runner platform, architecture, and exact framework revision; Cargo validates individual compiler/dependency/build-script fingerprints when reusing that older app cache. The exact current key remains the save key.

Tests cover domain calendar rules and relationships, smart-list commands, undo/redo, affected-entity cache updates, callback reentrancy, persistence failures, reminder deduplication, imports, and cloud replacement semantics. Selection tests verify that bulk operations remain atomic across undo, navigation resets anchors, and select-all matches visible completed-group behavior. Graph tests exercise the adapter against controlled service responses. The Pages workflow keeps one build path and a short browser editing/rendering check. That check drives real input, reads persisted state without flushing it through an export helper, and captures desktop and narrow layouts.

No application-specific frame-time or large-dataset performance budget has been established yet. Virtualization, retained row sources, cached icon geometry, theme trimming, and targeted state updates are implementation choices; they should not be read as measured performance guarantees.
