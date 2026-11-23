# Account workspaces

**Settings → Switch Microsoft account** opens the Microsoft account chooser. Each account has an independent task document on this device. Returning to an account restores its saved edits, settings, files, list order, and synchronization history. **Open local workspace** returns to the separate local document and disconnects the active Microsoft session.

The first connection can adopt the current local tasks. An account that already has a saved workspace always restores that workspace. Switching from one Microsoft account to a new account starts with an empty document; it cannot upload the previous account's tasks. The account key includes the Microsoft cloud, so equivalent subject IDs in different clouds are separate workspaces.

Disconnect and sign-out keep the currently displayed tasks available offline. They do not delete a workspace or its remote data. Reconnecting or selecting another account restores the corresponding document. Authentication still uses Microsoft's account chooser and the configured public client application; saved task documents do not constitute an authenticated session.

## Storage and transactions

The existing active document remains `ferroui.todo.v1` in the browser and `tasks.json` on desktop. Account snapshots use `ferroui.todo.workspaces.v1.document.<JSON account key>` in the service-side storage abstraction. The `null` account key denotes the local workspace. Native sidecars have hashed filenames and atomic replacement; browser document writes retain their synchronous compare-and-swap boundary.

A switch performs these operations in order:

1. Flush pending native editors and compare the mounted document with durable storage.
2. Checkpoint the outgoing document under its current account key.
3. Load and validate the incoming snapshot, or prepare the intended new document.
4. Persist a journal containing the exact before/after documents and account keys.
5. Save the incoming document through the host's ordinary persistence callback.
6. Publish its account binding and snapshot, acknowledge the browser storage baseline, and remove the journal.

Recovery completes the account binding only when the durable document matches the journal's after-image; it rolls back the binding when the before-image remains. An unrelated document causes a visible recovery error and preserves the journal. Recovery never overwrites the task document. Browser startup recovery, activation, synchronization, and restore use the same Web Lock. A stale tab also compares the workspace identity on every save, so identical JSON bytes in two accounts cannot authorize a cross-account write.

The live active document is authoritative when the selected account has not changed. Its archived snapshot is refreshed from that document. This preserves reminders marked delivered by the native headless helper while the desktop UI was closed.

Graph and Planner sidecars already include account keys. They survive switching and keep pending operations, remote identities, and conflict baselines separate. Cached Graph attachment references also encode their account; opening them requires that account. Native cleanup retains file generations referenced by inactive workspace snapshots as well as active tasks and pending cloud operations.

## Entity identity and UI state

Numeric task/list IDs belong to one document, not to the entire installation. An account replacement therefore clears undo/redo, task selection, task cursor, navigation history, pending rename/step drafts, category presentation, clipboard leases, and delivered-reminder identity state. Incoming sidebar order is retained. The host cancels active gestures before replacing the document, and drag origins also carry the document epoch.

The replacement saves before mutating the view model. A storage failure leaves the original document and its history intact. A successful switch publishes the new reactive cache and returns navigation to My Day. Formatting, custom wallpaper data, and other portable task fields are preserved by the document schema.

## Restore behavior

An explicit archive restore checkpoints the outgoing workspace and journals the exact local replacement. Browser automatic session selection or native durable sign-in resume is revoked before the task write. The imported document is bound to the local workspace; the original account's snapshot and synchronization history remain available. A crash after the task write cannot silently reconnect the imported copy to the previous account.

Restored copies retain the existing conservative connector settings: automatic sync and optional integrations are disabled. The user can open a saved Microsoft workspace or explicitly connect a local workspace afterward. Import never writes remote Microsoft tasks.

## Limits and verification

Workspaces are local caches in the current browser origin/profile or desktop data directory. They are not an OS-user access-control boundary. Task data is not encrypted by this layer; native Microsoft token persistence is separately encrypted using an OS-vault key. Clearing site data or the app's data directory removes local workspace snapshots and local files. Portable exports contain the currently open document, not every account's workspace or credentials.

Browser storage quota can prevent checkpointing a large document. Such a failure occurs before the account document is replaced and is surfaced to the user. The app does not discard another account's snapshot to free space.

Automated coverage includes account/local switching, legacy binding migration, preserved cloud sidecars, stale-tab rejection with byte-identical documents, validation/storage failures, crash recovery, restore resume revocation, inactive-file retention, and task-ID collisions across undo/clipboard/reminder/drag state. Live Microsoft account selection remains dependent on registration and user consent.
