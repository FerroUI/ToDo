# Microsoft accounts, synchronization, and backups

The browser application connects to Microsoft To Do through Microsoft Graph and, for work or school accounts, can show assigned Planner tasks. Tasks also work locally without an account. The repository ships an empty public client ID: the deployment owner must supply a real Microsoft application registration before configured sign-in is available. No signed-in Microsoft account was used for an authenticated end-to-end check during this implementation.

See [the integration review](integration-review.md) for the comparison with Microsoft's products and the remaining service gaps.

## Register and configure the browser application

1. In Microsoft Entra **App registrations**, register the application. To support both personal and work/school accounts, select the account type that permits organizational directories and personal Microsoft accounts.
2. Add a **Single-page application** platform. Register the exact canonical redirect URI **`https://ferroui.com/ToDo/`**, including case and trailing slash. If the separate GitHub Pages origin is used, register **`https://ferroui.github.io/ToDo/`** separately. Local development origins also need their own redirects.
3. Configure delegated Microsoft Graph permissions **`Tasks.ReadWrite`** and **`User.Read`**. Consent is subject to the organization's policies. The implemented Planner endpoints use the same task permission; the app does not request group administration or mailbox-reading permissions.
4. Supply the public configuration below, deploy, and select **Connect Microsoft account** in Settings. Never put a client secret, certificate private key, access token, or refresh token in this static application.

The registration must use the SPA redirect type for the browser authorization-code flow. Microsoft documents [authorization-code flow with PKCE and SPA redirects](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow).

### Public deployment configuration

The app reads `microsoft-config.json` beside its entry page. Its checked-in defaults are:

```json
{
  "clientId": "",
  "tenant": "common",
  "redirectUri": ""
}
```

Set `clientId` to the registration's public Application (client) ID. `tenant` accepts `common`, `organizations`, `consumers`, or a directory GUID and must agree with the registration's supported account types. An empty `redirectUri` derives the canonical current app address without query or fragment. An explicit redirect must match this app's origin and path. HTTPS is required except for local loopback development. A deployment configured for `ferroui.com` must be opened there; another origin needs its own configuration and registered redirect.

The Pages workflow accepts these **GitHub repository variables**:

| Repository variable | Build environment variable | JSON field |
| --- | --- | --- |
| `MICROSOFT_CLIENT_ID` | `TODO_MICROSOFT_CLIENT_ID` | `clientId` |
| `MICROSOFT_TENANT` | `TODO_MICROSOFT_TENANT` | `tenant` |
| `MICROSOFT_REDIRECT_URI` | `TODO_MICROSOFT_REDIRECT_URI` | `redirectUri` |

For this deployment, set `MICROSOFT_REDIRECT_URI` to `https://ferroui.com/ToDo/`. `scripts/build-browser.sh` runs `scripts/configure-microsoft.mjs` before preparing the offline site and asset hashes. With `TODO_MICROSOFT_CLIENT_ID` already set to the real public GUID, a local build can use:

```bash
TODO_MICROSOFT_TENANT=common \
TODO_MICROSOFT_REDIRECT_URI=https://ferroui.com/ToDo/ \
./scripts/build-browser.sh
```

An unconfigured deployment also accepts a public client ID in Settings for a separately registered installation. With deployment configuration present, users can connect directly without entering a client ID.

### Sign-in and sessions

Authentication uses vendored **MSAL Browser 5.30.0**, authorization-code flow with PKCE, MSAL's state and nonce validation, and `sessionStorage` token caching. Sign-in and interactive renewal use a full-page Microsoft redirect. The app restores the selected account, strips OAuth response parameters, and preserves safe application query parameters. Tokens are excluded from task documents and exports.

MSAL renews tokens silently when possible. A Graph 401 triggers token renewal; sessions needing interaction show a reconnect state and stop automatic requests that require sign-in. See Microsoft's [token lifetime and cache-policy guidance](https://learn.microsoft.com/en-us/entra/msal/javascript/browser/token-lifetimes).

| Account action | Result |
| --- | --- |
| **Reconnect Microsoft account** | Return to Microsoft to renew the selected account's authorization or consent. |
| **Disconnect this app** | Clear this app's selected MSAL session and stop cloud work; retain local tasks. Other Microsoft applications remain signed in. |
| **Sign out of Microsoft** | Use Microsoft's logout redirect for the selected account as well as clearing this app's session. |

Ordinary disconnect preserves the document's account binding so a different account cannot silently receive another account's tasks. Successful backup restoration creates an independent document and clears that binding. See [MSAL logout behavior](https://learn.microsoft.com/en-us/entra/msal/javascript/browser/logout).

## Synchronization behavior

**Sync automatically** is on by default for a connected account. The browser checks To Do changes on a five-second foreground interval after the preceding operation finishes; local saves request sync after a 900 ms debounce. Microsoft describes a [five-second To Do update cadence](https://support.microsoft.com/en-us/todo/syncing-your-microsoft-to-do-account). This implementation uses that foreground cadence without promising delivery while the browser is closed, suspended, offline, or throttled.

Hidden and offline tabs pause work. Visibility and connectivity changes resume it. **Sync now** and pull-to-refresh use the same serialized operation path. List/task delta queries avoid downloading unchanged collections; changed tasks hydrate checklist items, links, and attachment metadata. Expired delta cursors trigger a full read. Account-scoped delta snapshots are published only after every required page succeeds.

Transient errors back off to at most a 60-second scheduler interval. Authentication, permission, and merge conflicts stop automatic retries until an appropriate local change, explicit retry, or reconnect. Graph throttling honors `Retry-After`. Uncertain creation responses are recovered from recorded intent instead of blindly repeated.

### To Do data support

| Data | Implemented behavior |
| --- | --- |
| Lists | Import lists returned for the account; create, rename, and delete owned user lists. Preserve built-in Tasks and Flagged Email lists and retain ownership/sharing flags. |
| Tasks | Create, update, complete/reopen, delete, and move between eligible lists. Send changed fields and preserve independent remote edits. |
| Details | Synchronize title, plain-text note editing, importance, due date, reminder, supported recurrence, and category names. Preserve unsupported server properties during unrelated edits. |
| Steps | Create, rename, complete, and delete checklist items with stable local-to-remote mappings. |
| Files | Upload, download, and delete Graph attachments with an application limit of 25 MiB per file. Cache cloud bytes in IndexedDB on demand. Larger uploads use resumable sessions. |
| Source links | Show safe HTTP(S) linked resources supplied by Microsoft, including originating-service links. Preserve them during moves. |
| Flagged email | Project the Graph Flagged Email task list into its smart view. No mail-body access or arbitrary Outlook message flagging/unflagging. |
| Categories | Synchronize category names. Outlook's master category catalogue, palette, and colors are not fetched. |
| Local presentation | Keep My Day, hidden suggestions, themes, groups, ordering, appearance preferences, and ordinary local assignment labels on this device. |

The connection uses public [Graph v1.0 tasks](https://learn.microsoft.com/en-us/graph/api/resources/todotask?view=graph-rest-1.0), [task lists](https://learn.microsoft.com/en-us/graph/api/resources/todotasklist?view=graph-rest-1.0), and supported child collections. Sharing flags do not provide invitation or membership management in this connection.

Daily, weekly, weekdays, monthly, and yearly schedules can be written. Richer server recurrences, range bounds, HTML bodies, start times, intermediate status, and other properties outside the editor's model survive unrelated edits. Editing those particular values uses the local editor's representable form; this is not a full rich-text or arbitrary Graph recurrence editor.

### Planner

For a connected **work or school account**, enable **Settings → Connected apps → Planner**. This also enables **Assigned to me**. Assigned tasks support title, due date, notes, checklist, and completion edits. My Day is a local selection. Unsupported operations use **Open in Planner**, which opens the official Planner entry point rather than an invented per-task URL.

Unchanged Planner assignments are polled at most once per 60 seconds during automatic sync. Local edits, pending checklist recovery, and manual sync bypass that wait. Task and detail updates use their separate entity tags. A Planner permission or merge-conflict failure pauses that connector's automatic updates while ordinary To Do synchronization can continue; manual sync retries it. Temporary network and service failures use the scheduler's bounded backoff and recover automatically.

This follows the documented [Planner connection in Microsoft To Do](https://support.microsoft.com/en-us/planner/see-planner-tasks-in-microsoft-to-do). The [assigned Planner tasks endpoint](https://learn.microsoft.com/en-us/graph/api/planneruser-list-tasks?view=graph-rest-1.0) supports organizational accounts; access also depends on licensing and organization policy. Personal Microsoft accounts do not support this connection.

## Export and restore

| Settings action | Contents and use |
| --- | --- |
| **Export backup** | Existing task-document JSON: local lists, tasks, settings, and attachment references. File bytes are not included. |
| **Export backup with files** | Portable JSON containing the canonical local document, actual file bytes as base64, and SHA-256 per unique file. Cloud files are materialized through the connected account. Missing bytes cause an error instead of an incomplete portable backup. |
| **Export Microsoft To Do data** | Fresh, read-only Graph account export: accessible lists/tasks, original returned properties, hydrated child collections, files, and checksums. Independent of the current local projection; excludes Planner plans and mailboxes. |
| **Restore backup…** | Replace the local document from legacy task JSON or a portable backup. |
| **Restore Microsoft archive…** | Replace the local document with independent copies converted from this application's Graph archive. |

These are **application-owned JSON formats**. They do not read/write Outlook or Purview PST. Microsoft's [To Do overview](https://learn.microsoft.com/en-us/graph/todo-concept-overview) documents separate PST field mapping. Its [announced future To Do export](https://support.microsoft.com/en-us/planner/to-do-to-planner) had no published format or complete instructions at the review date; compatibility with that future format is not claimed.

```text
Portable envelope:
  kind: "ferroui.todo.backup"
  version: 1
  document: canonical application document
  files: [{ key, name, type, size, base64, sha256 }]

Microsoft Graph envelope:
  kind: "ferroui.microsoft-todo"
  version: 1
  exportedAt: ISO UTC timestamp
  lists: raw hydrated Graph list records containing tasks
  files: [{ key, source: { listId, taskId, attachmentId }, name, type, size, base64, sha256 }]
```

Opaque archive file keys replace account-bound cache keys. Tokens, MSAL state, account-profile wrappers, and synchronization sidecars are excluded. Raw Graph records retain their intrinsic IDs and source properties as data. Account-archive restoration retains original task fields in `microsoft_data`, preserving unsupported recurrence, HTML, start time, importance, and status details. Future upload seeds are limited to supported writable fields. Original list properties remain in the archive even when they have no local UI equivalent.

Limits are **25 MiB per file**, **128 MiB of decoded file contents**, **32 MiB for metadata/document JSON**, and **256 MiB for the archive file**. Base64 increases encoded size. Legacy reference-only backups cannot restore uncopied file bytes on another device. The file picker checks size before reading; restore validates version, relationships, duplicate/orphan references, metadata, canonical base64, and checksums before writing blobs.

### Restoration creates an independent local copy

Restoration replaces the current local document; export first if both documents are needed. Attachments receive fresh storage keys to avoid overwriting existing blobs. Rust validates the resulting document before mutation; guards detect local edits during asynchronous work. The document is imported once after all files are ready. Failure removes newly staged files and preserves the existing document.

Success sets automatic sync and Planner **off**, removes active cloud/list/provider bindings and assignment/flagged-email projection markers, disconnects this app's Microsoft session, and clears synchronization mappings. Source URLs remain as links. Restored flagged-email and Planner items become ordinary local tasks. The status reads **“Restored as a local copy. Microsoft sync is paused.”**

Importing never uploads to Microsoft. To publish the independent copies, connect the intended account and explicitly sync; this creates new tasks instead of updating original remote identities. Document replacement participates in local undo; undo does not reconnect a Microsoft session. Exported archives retain original source fields for inspection or other tools.

## Conflicts, moves, and recovery

The sidecar keeps separate local/remote baselines. Disjoint field edits merge; incompatible edits to the same field stop visibly. Conditional entity tags protect updates where available. Planner requires independent task/detail tags, as documented for [task updates](https://learn.microsoft.com/en-us/graph/api/plannertask-update?view=graph-rest-1.0) and [detail updates](https://learn.microsoft.com/en-us/graph/api/plannertaskdetails-update?view=graph-rest-1.0).

Before remote mutations and local replacement, the client checks that the captured document is current. Web Locks prevent overlapping cloud sync from tabs at the same origin. Account changes and restore/export also use the serialized queue. Browsers without Web Locks report that safe cloud sync is unavailable.

Moves preserve writable source properties, steps, links, and file bytes. The destination is populated before source deletion. The source is read again, attachment content is checked, and conditional deletion rejects a changed source. A late conflict retains both copies for review. Graph supplies no atomic transaction for an entire multi-request sync or account export: earlier operations can succeed before a later failure, and exports are not server-wide transactional snapshots.

Durable creation intents, correlations, upload progress, and a commit journal recover interrupted operations. Ambiguous outcomes stop for review. Deleted local files remain recoverable through attachment trash for undo; unreferenced trash is cleaned after its retention period. For conflicts, preserve a local backup, reconcile the reported Microsoft task fields, and retry. Clearing browser storage also removes local tasks and files and is not a normal recovery step.

## Platform and verification scope

Microsoft service integration, binary portable backups, and Graph archives are implemented in the **browser host**. The native desktop host currently provides local tasks and existing document export/import, without native Microsoft OAuth, Graph, or Planner synchronization. Its Settings link opens the browser app.

Reminders are checked while running and on resume. Browser notifications require permission and platform support. No background server, webhook receiver, or push service wakes a closed app. The implementation targets global `login.microsoftonline.com` and `graph.microsoft.com`; sovereign-cloud hosts are not configurable.

Run `node --test scripts/*.test.mjs` for the host/auth/Graph/Planner/archive fixtures. Tests exercise production modules with mocked MSAL and Graph, including delta recovery, files, moves, redirects, account isolation, serialization, and validation before restore. Rust tests cover document validation and domain capabilities. These checks do not establish that a particular registration, tenant consent policy, license, live account, or deployment has been verified; current CI and authenticated acceptance results must be checked separately.

Microsoft authentication code and Unicode time-zone data retain their licenses under `crates/todo-browser/wwwroot/vendor/`.
