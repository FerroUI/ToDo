# Native Microsoft services

The FerroUI desktop application uses its Rust MVVM interface and an isolated Node.js service process. The process runs the same Graph synchronization, Planner, connected-app, attachment, and archive modules as the browser application. Network requests and archive work do not block the UI dispatcher.

## Run and configure

Install **Node.js 22 or newer** and put `node` on `PATH`, or set `TODO_NODE_BINARY` to its absolute executable path. The desktop binary embeds its service code, Microsoft authentication library, and HTML parser. No npm install is needed on the user's computer. If the runtime is unavailable, settings explain the requirement; local task editing continues to work.

Register an application you control in Microsoft Entra. Add **Mobile and desktop applications → `http://localhost/`** to its redirect URIs. This is separate from the browser application's Single-page application redirect. The desktop host binds IPv4 loopback on an ephemeral port and uses `http://localhost:<port>/` for the individual authorization request. Microsoft treats the localhost port specially for native redirect matching. Do not supply a client secret to this public-client application. See [Microsoft's redirect URI documentation](https://learn.microsoft.com/en-us/entra/identity-platform/reply-url).

The repository's owner-run registration helper also describes both platforms; see [Microsoft setup](MICROSOFT.md). No registration or account consent is performed by the build.

| Environment variable | Meaning |
| --- | --- |
| `TODO_NODE_BINARY` | Node.js executable; defaults to `node`. |
| `TODO_MICROSOFT_CLIENT_ID` | Registered application's public client ID. It can also be entered in account settings. |
| `TODO_MICROSOFT_TENANT` | `common`, `organizations`, `consumers`, or a tenant GUID. Default: `common` for the global cloud, `organizations` for government clouds. |
| `TODO_MICROSOFT_CLOUD` | `global`, `usgov`, `usgovdod`, or `china`. The China environment explicitly refuses unsupported To Do services. |
| `TODO_MICROSOFT_NATIVE_REDIRECT_URI` | Optional registered loopback URI; defaults to `http://localhost/`. Only plain HTTP `localhost`, `127.0.0.1`, or `[::1]` without credentials, query, or fragment is accepted. |
| `TODO_DATA_DIR` | Optional private application data directory. Only one native instance may own a directory at a time. |

Example for a global public-client registration:

```sh
TODO_MICROSOFT_CLIENT_ID="your-application-guid" cargo run -p todo-desktop
```

The usual data directory is `%LOCALAPPDATA%/FerroUI/ToDo` on Windows, `~/Library/Application Support/FerroUI/ToDo` on macOS, and `$XDG_DATA_HOME/FerroUI/ToDo` or `~/.local/share/FerroUI/ToDo` on Linux. Public configuration and sync mappings persist there; authentication tokens do not. Unix application directories use owner-only permissions. The OS-owned directory lock is released after a crash, so no stale PID-file cleanup is required.

## Sign-in and account lifecycle

Sign-in opens the system browser and uses the official **MSAL Node 7.2.3** public-client implementation with authorization code + PKCE. The single-use loopback listener validates the callback's state, host, path, and method, and closes after success or a three-minute timeout. Identity token nonce validation binds the result to the request. Tokens never cross the UI bridge and never appear in status messages or service logs. See [MSAL Node token acquisition](https://learn.microsoft.com/en-us/entra/msal/javascript/node/acquire-token-requests).

The MSAL token cache stays in the service process's memory. Reconnect after restarting the application. Closing into the supported tray mode retains the running session; **Quit To Do**, an ordinary close without background mode, or process termination ends it. Disconnect cancels outstanding work, destroys the service session, clears account presentation, and keeps local tasks. Sign out additionally opens Microsoft's logout endpoint. This release does not claim OS credential-vault persistence or automatic sign-in after a full process restart.

Task sync requests `Tasks.ReadWrite` and `User.Read`. Outlook category catalogue access requests `MailboxSettings.Read` only when enabled. Optional OneDrive presentation sync requests `Files.ReadWrite.AppFolder` only when enabled. An optional integration's consent or token failure does not invalidate the base task session. Government clouds use fixed authority/Graph pairs and Graph-qualified scopes; personal Microsoft accounts are rejected there.

## Available native actions

Account settings, manual refresh, automatic task synchronization, supported Planner assignments, optional Outlook category colours, and optional OneDrive presentation sync use the common service implementations. Microsoft service restrictions and tenant policies still apply; see [the integration review](integration-review.md).

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

The native reminder clock runs every 30 seconds while the application is running, including when hidden in supported tray mode. Linux uses the desktop notification D-Bus service. Windows submits a WinRT toast under `FerroUI.ToDo` through a fixed PowerShell helper. macOS uses the system scripting host's notification facility; notification attribution and permission belong to that host in this release. Task text is passed as data, never interpolated into executable scripts. A failed delivery restores a hidden window and reports the failure. The test-reminder button reports that a request was sent, because OS focus modes and notification preferences can suppress display.

When completion sounds are enabled, the native host synthesizes the same short two-tone chime used by the browser. Playback runs off the UI thread: Windows uses `PlaySoundW`, macOS uses `afplay`, and Linux tries `pw-play`, `paplay`, then `aplay` when available. Playback failure leaves the completed task unchanged.

This is running-process background support. It does not register OS wake timers, start automatically at login, deliver after the process has quit, or wake a powered-off device. Native tray and notification acceptance on Windows/macOS require platform validation; the current development verification covers the Linux build and the service-process tests.

## Consistency and recovery

Rust owns the live document. The service process communicates through inherited private pipes with bounded, length-prefixed messages. UI callbacks execute on FerroUI's dispatcher. Cloud updates and imports use a document compare-and-swap; an edit made while an archive or request is being prepared prevents the stale result from replacing it. Staged attachment writes are rolled back on a failed restore.

The common Graph commit journal and request reconciliation are retained. Restarting the worker recovers a pending metadata commit before accepting another command. Cancellation advances a generation and kills both service and parser children; stale callbacks are rejected. A failed service process clears the UI's account connection so a dead in-memory token cache cannot be presented as connected. PST decode failures remain isolated from a healthy Microsoft service session.

## Development checks and bundled code

```sh
cargo check -p todo-desktop --locked
cargo test -p todo-interop --locked
node --test scripts/native-services.test.mjs
```

The native tests exercise real child-process framing and shared modules: loopback request validation, inert HTML conversion, file/archive round trips, a concurrent-edit restore rejection with staged-file cleanup, attachment undo retention, missing-session failures, and authentic PST export. Interop tests verify the allocator's failure/concurrency behavior and execute valid and malicious-count PSTs in disposable, budgeted child processes. They do not log in to a live Microsoft account. Account/tenant integration and Windows/macOS OS UI acceptance must be checked with the owner's registered application.

The native bundle includes Microsoft's MIT-licensed [MSAL JavaScript](https://github.com/AzureAD/microsoft-authentication-library-for-js) and the MIT-licensed [parse5](https://github.com/inikulin/parse5) 8.0.0 HTML parser. License text is retained in `crates/todo-desktop/native/vendor/`. The parser is inert: Graph/PST HTML is converted to plain notes without loading remote resources. All service assets are embedded at build time and extracted into a content-versioned directory, so a released native binary does not depend on source checkout files.
