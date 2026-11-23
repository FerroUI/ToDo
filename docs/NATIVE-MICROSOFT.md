# Native Microsoft services

The FerroUI desktop application uses its Rust MVVM interface and an isolated Node.js service process. The process runs the same Graph synchronization, Planner, connected-app, attachment, and archive modules as the browser application. Network I/O, file preparation, and PST decoding run off the UI dispatcher; final document validation and commits run on the dispatcher.

## Run and configure

The installed packages include a verified Node.js runtime. Developer builds can use **Node.js 22 or newer** on `PATH`; `TODO_NODE_BINARY` explicitly selects another executable. Runtime discovery checks only the installed executable’s adjacent runtime directory or the macOS app Resources directory before falling back to `PATH`, never the working directory. The desktop binary embeds its service code, Microsoft authentication library, and HTML parser. No npm install is needed on the user's computer. If the runtime is unavailable, settings explain the requirement; local task editing continues to work.

Register an application you control in Microsoft Entra. Add **Mobile and desktop applications → `http://localhost/`** to its redirect URIs. This is separate from the browser application's Single-page application redirect. The desktop host binds IPv4 loopback on an ephemeral port and uses `http://localhost:<port>/` for the individual authorization request. Microsoft treats the localhost port specially for native redirect matching. Do not supply a client secret to this public-client application. See [Microsoft's redirect URI documentation](https://learn.microsoft.com/en-us/entra/identity-platform/reply-url).

The repository's owner-run registration helper also describes both platforms; see [Microsoft setup](MICROSOFT.md). No registration or account consent is performed by the build.

| Environment variable | Meaning |
| --- | --- |
| `TODO_NODE_BINARY` | Explicit Node.js executable override; otherwise prefer the installed runtime, then `node` on `PATH`. |
| `TODO_MICROSOFT_CLIENT_ID` | Registered application's public client ID. It can also be entered in account settings. |
| `TODO_MICROSOFT_TENANT` | `common`, `organizations`, `consumers`, or a tenant GUID. Default: `common` for the global cloud, `organizations` for government clouds. |
| `TODO_MICROSOFT_CLOUD` | `global`, `usgov`, `usgovdod`, or `china`. The China environment explicitly refuses unsupported To Do services. |
| `TODO_MICROSOFT_NATIVE_REDIRECT_URI` | Optional registered loopback URI; defaults to `http://localhost/`. Only plain HTTP `localhost`, `127.0.0.1`, or `[::1]` without credentials, query, or fragment is accepted. |
| `TODO_DATA_DIR` | Optional private application data directory. Only one native instance may own a directory at a time. |

Example for a global public-client registration:

```sh
TODO_MICROSOFT_CLIENT_ID="your-application-guid" cargo run -p todo-desktop
```

The usual data directory is `%LOCALAPPDATA%/FerroUI/ToDo` on Windows, `~/Library/Application Support/FerroUI/ToDo` on macOS, and `$XDG_DATA_HOME/FerroUI/ToDo` or `~/.local/share/FerroUI/ToDo` on Linux. Public configuration and sync mappings persist there, separately from an encrypted account cache. The cache's encryption key is held in the OS credential store; plaintext tokens are not written to files. Unix application directories use owner-only permissions. The OS-owned directory lock is released after a crash, so no stale PID-file cleanup is required.

## Sign-in and account lifecycle

Sign-in opens the system browser and uses the official **MSAL Node 7.2.3** public-client implementation with authorization code + PKCE. The single-use loopback listener validates the callback's state, host, path, and method, and closes after success or a three-minute timeout. Identity token nonce validation binds the result to the request. Tokens never cross the UI bridge and never appear in status messages or service logs. See [MSAL Node token acquisition](https://learn.microsoft.com/en-us/entra/msal/javascript/node/acquire-token-requests).

The service process uses an MSAL cache plugin to save an AES-256-GCM encrypted cache and resume the explicitly selected account after restart. A random 32-byte encryption key is held in Windows Credential Manager, macOS Keychain, or Linux Secret Service. These stores are selected by the source-built [`keyring` 4.2.0 v1 adapter](https://docs.rs/keyring/4.2.0/keyring/v1/index.html) in the reusable `todo-credentials` crate. The desktop executable exposes a bounded helper mode; requests and key bytes travel only through private process pipes, with a deadline and generic errors. No Node native addon or runtime npm install is required.

Cache authentication binds the app-data profile, application client ID, tenant, and cloud. The selected account identity is inside the encrypted payload. Startup accepts only that unambiguous identity from MSAL's restored accounts; it never chooses the first cached account. A missing, locked, or unavailable credential store leaves sign-in usable in a visibly reported session-only mode. Damaged or mismatched cache data cannot resume a connection. There is no plaintext persistence fallback. Real OS-vault acceptance has not been exercised in this environment; controlled persistence and failure tests are separate from platform acceptance.

An explicit application client ID entered in account settings survives restart and reconnect instead of being overwritten by an environment default. Clearing that setting restores the environment default. A credential-store or cache-integrity failure remains visible for the current session; deliberate connection or reconnection retries persistence instead of background synchronization repeatedly probing the store.

The plaintext cache envelope is capped at 4 MiB before encryption. Helper requests are limited to 4 KiB and responses to 1 KiB; the helper has its own six-second watchdog, in addition to the worker's timeout. Unix marker/cache writes flush their file and, where supported, parent directory. Node does not expose an equivalent Windows directory-flush handle. A failed partial write attempts to disable resume; an unresolved storage error is reported explicitly instead of promising session-only behavior.

Closing into supported tray mode retains the running session. **Quit To Do**, ordinary close without tray mode, and process termination stop continuous synchronization while preserving a valid encrypted cache for the next launch. The separately enabled OS reminder schedule can launch the headless helper after exit. Disconnect first durably disables account restoration, invalidates pending cache writes, and then removes the saved cache/key; local tasks remain. Backup restore disables saved sign-in before committing its independent document. Sign out additionally opens Microsoft's logout endpoint. Failed cache deletion cannot by itself reactivate a disabled session. Expired or revoked Microsoft authorization can still require an interactive reconnect, as described in [MSAL token caching](https://learn.microsoft.com/en-us/entra/msal/javascript/node/caching).

Task sync requests `Tasks.ReadWrite` and `User.Read`. Outlook category catalogue access requests `MailboxSettings.Read` only when enabled. Create/recolor/delete requires separate explicit `MailboxSettings.ReadWrite` consent; it does not grant mailbox-content access. Optional OneDrive presentation sync requests `Files.ReadWrite.AppFolder` only when enabled. An optional integration's consent or token failure does not invalidate the base task session. Government clouds use fixed authority/Graph pairs and Graph-qualified scopes; personal Microsoft accounts are rejected there.

## Available native actions

Account settings, manual refresh, automatic task synchronization, supported Planner assignments, optional Outlook category colours and separately consented catalogue management, and optional OneDrive presentation sync use the common service implementations. Microsoft service restrictions and tenant policies still apply; see [the integration review](integration-review.md).

Native file pickers support local attachments, saving local or cloud attachment content, portable backups with files, Microsoft Graph archive import/export, and Outlook PST export. Deleting a local attachment retains recoverable bytes for undo before cleanup. Portable restore uses the common validation and file hash checks, restores a local copy, and pauses Microsoft sync. Native PST import first converts an Outlook task store in an isolated parser process, then applies the common timezone normalization and portable restore path.

Every native PST parser child enables a **1.5 GiB cumulative Rust allocation budget before reading input**. This covers the decoder's unchecked `Vec` allocations: exceeding the remaining budget aborts only that child. Deallocation does not refund the budget, and reallocation counts its complete new size. This is a conservative cumulative allocation ceiling, not a resident-memory measurement. The normal UI process leaves the budget disabled, adding only one atomic load per allocation and no headers or free-time bookkeeping.

The common child-process isolation is supplemented by platform controls:

| Platform | Native PST import isolation |
| --- | --- |
| Linux | Cumulative allocation budget plus checked 1.5 GiB `RLIMIT_AS` and parent-death termination. |
| Windows | Cumulative allocation budget plus a private Job Object established before input is sent. The job caps process commit at 1.5 GiB and terminates the child when its handle closes; the child verifies the active cap. Failure to establish the job refuses import. |
| macOS and other native platforms | Cumulative allocation budget in the isolated parser process; no unsupported claim that macOS `RLIMIT_AS` provides an equivalent process-wide memory limit. |

All platforms use a 60-second parser watchdog, bounded input/output, and cancellation on app shutdown. On budget exhaustion, the host reports a failed import and keeps the live document unchanged. The allocation budget applies to Rust allocations; it is not a general sandbox for arbitrary native code. The PST decoder uses Rust allocations for the untrusted counts this budget addresses.

PST source files are limited to 256 MiB. The portable attachment rules also apply: 25 MiB per file and 128 MiB in total. A store that exceeds the parser's allocation or OS memory limits must be split into smaller task-only exports. See [Outlook PST interoperability](OUTLOOK-PST.md) for preserved properties, conversion limits, and writer validation. Windows Job Object behavior follows [Microsoft's process limit documentation](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_extended_limit_information).

## Background operation and reminders

**Keep running in background** is opt-in and appears only when FerroUI supplies a usable tray icon with a menu. Linux additionally requires an actual registered StatusNotifier host on the session bus; a session bus alone is insufficient. On desktops without an indicator host, closing the main window exits normally. If the host disappears while the app is hidden, the app restores its window instead of leaving an unreachable process.

With the setting enabled, an ordinary window close commits edits and hides the window. The tray provides **Open To Do**, **Sync now**, and **Quit To Do**. Explicit Quit and OS/application shutdown close the process. Automatic sync continues while hidden if auto-sync is enabled. It uses the shared serialized synchronization loop, approximately five-second polling after successful work, a short debounce after edits, and bounded failure backoff. Local document comparisons avoid unnecessary sync merely because transient UI state changed.

The native reminder clock runs every 30 seconds while the application is running, including when hidden in supported tray mode. Linux uses the desktop notification D-Bus service. Windows submits a WinRT toast through a fixed PowerShell helper, using the installed MSIX application identity when present or the per-user `FerroUI.ToDo` identity for an unpackaged executable. macOS uses the system scripting host's notification facility; notification attribution and permission belong to that host in this release. Task text is passed as data, never interpolated into executable scripts. A failed delivery restores a hidden window and reports the failure. The test-reminder button reports that a request was sent, because OS focus modes and notification preferences can suppress display.

When completion sounds are enabled, the native host synthesizes the same short two-tone chime used by the browser. Playback runs off the UI thread: Windows uses `PlaySoundW`, macOS uses `afplay`, and Linux tries `pw-play`, `paplay`, then `aplay` when available. Playback failure leaves the completed task unchanged.

### Reminders after the window closes

**Remind me after this app closes** is a separate per-workspace opt-in. It schedules the nearest pending reminder through a per-user systemd timer/service on Linux, a LaunchAgent in the macOS GUI domain, or Windows Task Scheduler with an interactive user token and catch-up enabled. A successful document save coalesces and reconciles that registration off the UI dispatcher. Disabling the setting, completing/deleting the last pending reminder, or activating a workspace with no scheduled work removes it. Closing resubmits the final document and flushes scheduler work.

The OS invokes `todo-desktop --deliver-reminders --data-dir <directory>` before GUI, Node, authentication or cloud initialization. The helper acquires the same application-directory lock as the UI. If the UI already owns it, the helper exits without touching data. It strictly loads the active `tasks.json`, sends due reminders, persists each accepted delivery, and plans the next event. It never replaces malformed data with an empty document. Account startup trusts that active file, so a stale profile snapshot cannot resurrect an acknowledged reminder.

A delivery failure remains pending and records a visible diagnostic. A crash between OS acceptance and durable acknowledgement can cause a repeated notification; the implementation favors retaining a reminder over silently discarding it. OS acceptance is not proof of visible display when focus modes, user permissions or notification services suppress it. Jobs require an available logged-in session, catch up when their platform permits, and cannot wake a powered-off device. Only the active persisted workspace is scheduled; inactive accounts are not background-synchronized or scanned.

Portable, Graph and PST restoration clears this machine opt-in and native keep-running. Switching existing profiles within the same installation retains each profile's explicit preference. OS registration and actual notification display remain platform acceptance checks, separate from deterministic scheduler/locking/delivery tests.

To remove a registration during uninstall, close the app and run:

```sh
todo-desktop --disable-background-reminders --data-dir /absolute/path/to/FerroUI/ToDo
```

`--reconcile-reminders` is an internal recovery/rescheduling command. These helper modes do not perform Microsoft sign-in or cloud synchronization.

## Consistency and recovery

Rust owns the live document. The service process communicates through inherited private pipes with bounded, length-prefixed messages. UI callbacks execute on FerroUI's dispatcher. Account/local documents have independent durable workspaces and exact switch/import journals; entity-scoped undo, clipboard leases, selection and reminder ledgers are cleared on activation. Cloud updates and imports use a document compare-and-swap; an edit made while an archive or request is being prepared prevents the stale result from replacing it. Staged attachment writes are rolled back on a failed restore.

The common Graph commit journal and request reconciliation are retained. Restarting the worker recovers a pending metadata commit before accepting another command. Cancellation advances a generation and kills both service and parser children; stale callbacks are rejected. A failed service process clears the UI's account connection until a subsequent worker has actually restored a valid saved account. PST decode failures remain isolated from a healthy Microsoft service session. Disconnect/restore revocation is separate from ordinary process shutdown, so ending a process does not masquerade as forgetting the saved account.

## Development checks and bundled code

```sh
cargo check -p todo-desktop --locked
cargo test -p todo-interop -p todo-credentials --locked
cargo test -p todo-desktop --lib --locked
node --test scripts/native-services.test.mjs scripts/native-cache.test.mjs scripts/native-auth.test.mjs
```

The native tests exercise real child-process framing and shared modules: loopback request validation, inert HTML conversion, file/archive round trips, a concurrent-edit restore rejection with staged-file cleanup, attachment undo retention, missing-session failures, and authentic PST export. A real worker restore test verifies that a failed resume-disabled marker prevents document replacement and removes staged files. Cache tests exercise authenticated encryption, account/configuration isolation, partial writes, reconnect recovery, and late completion after disconnect. Authentication fixtures use the real vendored MSAL cache while replacing only browser, network, and OS-vault boundaries; they cover restart, rejected optional consent, cache rollback, and explicit client-ID precedence. Credential-protocol tests use an injected store and verify error redaction. Interop tests execute valid and malicious-count PSTs in disposable, budgeted child processes. No test logs in to a live Microsoft account. Account/tenant integration and actual credential-store, file-picker, tray, notification, and audio acceptance on each OS remain separate validation requirements.

The native bundle includes Microsoft's MIT-licensed [MSAL JavaScript](https://github.com/AzureAD/microsoft-authentication-library-for-js) and the MIT-licensed [parse5](https://github.com/inikulin/parse5) 8.0.0 HTML parser. License text is retained in `crates/todo-desktop/native/vendor/`. The parser is inert: Graph/PST HTML is sanitized into the allowlisted rich-note model and searchable plain projection without loading remote resources. All service assets are embedded at build time and extracted into a content-versioned directory, so a released native binary does not depend on source checkout files.

## Distribution

[Native packaging](NATIVE-PACKAGING.md) documents verified bundled runtimes, Linux/macOS/Windows layouts, distribution artifacts, signing and notarization hooks, dependency/license manifests, scheduler cleanup and the platform build matrix. Native distribution uses the existing `dist-unwind` profile so a failed deferred OS drag-transfer read can be contained before any task mutation. Browser compilation retains its own profile.
