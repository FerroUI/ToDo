# Microsoft accounts, synchronization, and backups

The browser and desktop applications connect to Microsoft To Do through Microsoft Graph and, for work or school accounts, can show assigned Planner tasks. Optional connections add Outlook category colors and synchronization of this app's presentation settings through OneDrive. Tasks also work locally without an account. The repository ships an empty public client ID: the deployment owner must supply a real Microsoft application registration before configured sign-in is available. No signed-in Microsoft account was used for an authenticated end-to-end check during this implementation.

See [the integration review](integration-review.md) for the comparison with Microsoft's products and the remaining service gaps.

## Register and configure the application

1. In Microsoft Entra **App registrations**, register the application. To support both personal and work/school accounts, select the account type that permits organizational directories and personal Microsoft accounts.
2. Add a **Single-page application** platform. Register the exact canonical redirect URI **`https://ferroui.com/ToDo/`**, including case and trailing slash. If the separate GitHub Pages origin is used, register **`https://ferroui.github.io/ToDo/`** separately. Local development origins also need their own redirects.
3. For the desktop host, add **Mobile and desktop applications** with **`http://localhost/`**. Keep this separate from the SPA redirect. The native worker binds an ephemeral loopback port, opens the system browser, and exchanges the returned code using PKCE. See [native setup](NATIVE-MICROSOFT.md) and Microsoft's [redirect URI rules](https://learn.microsoft.com/en-us/entra/identity-platform/reply-url).
4. Configure delegated Microsoft Graph permissions **`Tasks.ReadWrite`** and **`User.Read`**. The implemented Planner endpoints use the same task permission. Optional features use **`MailboxSettings.Read`** for the Outlook category catalogue and **`Files.ReadWrite.AppFolder`** for this app's OneDrive settings. The app requests these two permissions incrementally when the respective feature is enabled. Organization consent policies still apply; no group administration, general mail reading, or unrestricted file permission is requested.
5. Supply the public configuration below, deploy, and select **Connect Microsoft account** in Settings. Never put a client secret, certificate private key, access token, or refresh token in this application.

The registration must use the SPA redirect type for the browser authorization-code flow. Microsoft documents [authorization-code flow with PKCE and SPA redirects](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow). Global registrations may support personal and organizational accounts; GCC High and DoD registrations use organizational accounts in their own cloud.

### Repeatable owner-run registration

`scripts/register-microsoft-app.mjs` uses an already authenticated Azure CLI session. Its default operation validates the selected tenant/cloud, discovers Microsoft's enabled delegated permission IDs, and prints a proposed registration and public configuration. It does not sign in, switch clouds, create secrets, or grant consent. Run it with the intended directory GUID:

```bash
node scripts/register-microsoft-app.mjs --tenant TENANT_GUID
node scripts/register-microsoft-app.mjs --tenant TENANT_GUID --apply
```

`--apply` explicitly creates the registration. To update a known registration, add `--application-id CLIENT_GUID`; unrelated existing redirects, permissions, and API settings are retained. The helper never adopts an application by matching its display name. `--cloud global|usgov|usgovdod`, `--accounts`, repeated `--spa-uri`/`--native-uri`, and `--display-name` customize the plan. `--output PATH` can write the resulting public configuration after application. Run `--help` for the complete syntax. The defaults register `https://ferroui.com/ToDo/` as a SPA and `http://localhost/` as a native public-client redirect.

Permission IDs are resolved from the selected cloud's Microsoft Graph service principal, using the [service-principal API](https://learn.microsoft.com/en-us/graph/api/serviceprincipal-list?view=graph-rest-1.0); registration uses the public [application API](https://learn.microsoft.com/en-us/graph/api/application-post-applications?view=graph-rest-1.0). A registration accepting personal accounts requests access-token version 2, as required by the [application API settings](https://learn.microsoft.com/en-us/graph/api/resources/apiapplication?view=graph-rest-1.0). Only mocked helper tests and `--help` have been executed during this implementation; no registration or consent was created.

### Public deployment configuration

The app reads `microsoft-config.json` beside its entry page. Its checked-in defaults are:

```json
{
  "clientId": "",
  "tenant": "common",
  "redirectUri": "",
  "cloud": "global"
}
```

Set `clientId` to the registration's public Application (client) ID. `tenant` accepts `common`, `organizations`, `consumers`, or a directory GUID and must agree with the registration's supported account types. National clouds reject `consumers` and normalize `common` to `organizations`. An empty browser `redirectUri` derives the canonical current app address without query or fragment. An explicit redirect must match this app's origin and path. HTTPS is required except for local loopback development. A deployment configured for `ferroui.com` must be opened there; another origin needs its own configuration and registered redirect. Unknown configuration fields, arbitrary authority/Graph URLs, and secret fields are rejected.

The Pages workflow accepts these **GitHub repository variables**:

| Repository variable | Build environment variable | JSON field |
| --- | --- | --- |
| `MICROSOFT_CLIENT_ID` | `TODO_MICROSOFT_CLIENT_ID` | `clientId` |
| `MICROSOFT_TENANT` | `TODO_MICROSOFT_TENANT` | `tenant` |
| `MICROSOFT_REDIRECT_URI` | `TODO_MICROSOFT_REDIRECT_URI` | `redirectUri` |
| `MICROSOFT_CLOUD` | `TODO_MICROSOFT_CLOUD` | `cloud` |

For this deployment, set `MICROSOFT_REDIRECT_URI` to `https://ferroui.com/ToDo/`. `scripts/build-browser.sh` runs `scripts/configure-microsoft.mjs` before preparing the offline site and asset hashes. With `TODO_MICROSOFT_CLIENT_ID` already set to the real public GUID, a local build can use:

```bash
TODO_MICROSOFT_TENANT=common \
TODO_MICROSOFT_CLOUD=global \
TODO_MICROSOFT_REDIRECT_URI=https://ferroui.com/ToDo/ \
./scripts/build-browser.sh
```

An unconfigured deployment also accepts a public client ID in Settings for a separately registered installation. With deployment configuration present, users can connect directly without entering a client ID. A local client-ID override retains the deployment's selected cloud. Native configuration uses the same client/tenant/cloud environment variables and a separate `TODO_MICROSOFT_NATIVE_REDIRECT_URI`; it must not reuse the HTTPS SPA redirect.

### Cloud availability and routing

Identity and Graph origins are a fixed pair chosen by `cloud`, following Microsoft's [national cloud deployments](https://learn.microsoft.com/en-us/graph/deployments). Tokens, account bindings, mappings, category caches, and presentation replicas cannot cross that boundary.

| `cloud` | Identity authority | Graph resource | App service availability |
| --- | --- | --- | --- |
| `global` | `https://login.microsoftonline.com` | `https://graph.microsoft.com` | Personal and organizational To Do; organizational Planner; optional categories and OneDrive settings. GCC moderate uses this environment. |
| `usgov` | `https://login.microsoftonline.us` | `https://graph.microsoft.us` | GCC High organizational services, subject to tenant licensing and permissions. |
| `usgovdod` | `https://login.microsoftonline.us` | `https://dod-graph.microsoft.us` | DoD organizational services, subject to tenant licensing and permissions. |
| `china` | `https://login.chinacloudapi.cn` | `https://microsoftgraph.chinacloudapi.cn` | Recognized configuration; this app's To Do, Planner, category, and presentation connections are unavailable. Local tasks remain available. |

The public [To Do task-list](https://learn.microsoft.com/en-us/graph/api/todo-list-lists?view=graph-rest-1.0), [Planner assignments](https://learn.microsoft.com/en-us/graph/api/planneruser-list-tasks?view=graph-rest-1.0), and [master-category](https://learn.microsoft.com/en-us/graph/api/outlookuser-list-mastercategories?view=graph-rest-1.0) endpoint availability tables exclude China. The app reports that boundary before authentication or task requests instead of substituting global endpoints. National-cloud Graph scopes name the selected resource explicitly; foreign-origin pagination and upload URLs are rejected.

Service handoffs also follow the selected cloud. Global accounts open Microsoft To Do and Planner. GCC High/DoD To Do opens the appropriate Outlook web entry, consistent with Microsoft's [account support guidance](https://support.microsoft.com/en-us/outlook/which-accounts-can-i-use-microsoft-to-do-with). GCC High Planner opens `tasks.office365.us`; DoD uses the documented Microsoft 365 portal rather than inventing a deep link. Hosts come from the [GCC High](https://learn.microsoft.com/en-us/microsoft-365/enterprise/microsoft-365-u-s-government-gcc-high-endpoints?view=o365-worldwide) and [DoD](https://learn.microsoft.com/en-us/microsoft-365/enterprise/microsoft-365-u-s-government-dod-endpoints?view=o365-worldwide) endpoint lists.

### Sign-in and sessions

Authentication uses vendored **MSAL Browser 5.30.0**, authorization-code flow with PKCE, MSAL's state and nonce validation, and `sessionStorage` token caching. Sign-in and interactive renewal use a full-page Microsoft redirect. The app restores the selected account, strips OAuth response parameters, and preserves safe application query parameters. Tokens are excluded from task documents and exports.

Desktop uses vendored **MSAL Node 7.2.3** in a Node.js 22+ child worker and a system-browser loopback authorization-code flow. A bounded AES-256-GCM encrypted cache can resume the explicitly selected account after restart. Its random encryption key is held in Windows Credential Manager, macOS Keychain, or Linux Secret Service through a source-built Rust helper. Cache authentication binds the app-data profile, client ID, tenant, and cloud; an ambiguous or mismatched cached account is not selected. An unavailable vault falls back visibly to session-only authentication, without plaintext token files. Disconnect and backup restoration disable saved-account restoration before cache cleanup. No client secret is required. See [native service details](NATIVE-MICROSOFT.md) for lifecycle and platform-verification limits.

MSAL renews tokens silently when possible. A Graph 401 triggers token renewal; sessions needing interaction show a reconnect state and stop automatic requests that require sign-in. See Microsoft's [token lifetime and cache-policy guidance](https://learn.microsoft.com/en-us/entra/msal/javascript/browser/token-lifetimes).

An optional category/OneDrive permission failure pauses that feature and leaves the ordinary task session usable. A canceled browser consent redirect can restore only the previously explicitly selected cached account, using a one-shot, short-lived marker bound to the exact registration, tenant, cloud, redirect, and requested optional scope. It cannot select another cached account or resurrect a disconnected session. Success and disconnect clear the marker. An optional-only token request cannot clear a reconnect requirement raised by the base task permission. Native interactive consent also requests the base scopes, so a successful validated grant can renew both permissions and clear that requirement.

| Account action | Result |
| --- | --- |
| **Reconnect Microsoft account** | Return to Microsoft to renew the selected account's authorization or consent. |
| **Disconnect this app** | Clear this app's selected MSAL session and stop cloud work; retain local tasks. Other Microsoft applications remain signed in. |
| **Sign out of Microsoft** | Use Microsoft's logout redirect for the selected account as well as clearing this app's session. |

Ordinary disconnect preserves the document's account binding so a different account cannot silently receive another account's tasks. Successful backup restoration creates an independent document and clears that binding. See [MSAL logout behavior](https://learn.microsoft.com/en-us/entra/msal/javascript/browser/logout).

## Synchronization behavior

**Sync automatically** is on by default for a connected account. The browser checks To Do changes on a five-second foreground interval after the preceding operation finishes; local saves request sync after a 900 ms debounce. Microsoft describes a [five-second To Do update cadence](https://support.microsoft.com/en-us/todo/syncing-your-microsoft-to-do-account). This implementation uses that foreground cadence without promising delivery while the browser is closed, suspended, offline, or throttled.

Hidden and offline tabs pause work. Visibility and connectivity changes resume it. **Sync now** and pull-to-refresh use the same serialized operation path. List/task delta queries avoid downloading unchanged collections; changed tasks hydrate checklist items, links, and attachment metadata. Expired delta cursors trigger a full read. Account-scoped delta snapshots are published only after every required page succeeds.

The desktop worker schedules automatic sync while its window is active or supported background operation is enabled. With a supported tray and the explicit background setting enabled, hiding the native window keeps that worker and the reminder clock alive. Quit, process exit, sleep, or loss of network still interrupts execution; this is not a server-side scheduler or closed-process push service.

Transient errors back off to at most a 60-second scheduler interval. Authentication, permission, and merge conflicts stop automatic retries until an appropriate local change, explicit retry, or reconnect. Graph throttling honors `Retry-After`. Uncertain creation responses are recovered from recorded intent instead of blindly repeated.

### To Do data support

| Data | Implemented behavior |
| --- | --- |
| Lists | Import lists returned for the account; create, rename, and delete owned user lists. Preserve built-in Tasks and Flagged Email lists and retain ownership/sharing flags. |
| Tasks | Create, update, complete/reopen, delete, and move between eligible lists. Send changed fields and preserve independent remote edits. |
| Details | Synchronize title, plain-text note editing, importance, due date, reminder, supported recurrence, and category names. Preserve unsupported server properties during unrelated edits. |
| Steps | Create, rename, complete, and delete checklist items with stable local-to-remote mappings. |
| Files | Upload, download, and delete Graph attachments with an application limit of 25 MiB per file. Cache cloud bytes on demand in browser IndexedDB or the native application-data directory. Larger uploads use resumable sessions. |
| Source links | Show safe HTTP(S) linked resources supplied by Microsoft, including originating-service links. Preserve them during moves. |
| Flagged email | Project the Graph Flagged Email task list into its smart view. No mail-body access or arbitrary Outlook message flagging/unflagging. |
| Categories | Synchronize task category names. An optional read-only Outlook master catalogue supplies names and semantic colors for the native Rust picker. |
| Presentation | My Day membership/exclusions, groups, list/sidebar/task order, list appearance, and supported preferences remain local unless the separate OneDrive device-settings connection is enabled. Local assignee labels and Microsoft's private metadata are not synchronized by that connection. |

The connection uses public [Graph v1.0 tasks](https://learn.microsoft.com/en-us/graph/api/resources/todotask?view=graph-rest-1.0), [task lists](https://learn.microsoft.com/en-us/graph/api/resources/todotasklist?view=graph-rest-1.0), and supported child collections. Sharing flags do not provide invitation or membership management in this connection.

Daily, weekly, weekdays, monthly, and yearly schedules can be written. Richer server recurrences, range bounds, HTML bodies, start times, intermediate status, and other properties outside the editor's model survive unrelated edits. Editing those particular values uses the local editor's representable form; this is not a full rich-text or arbitrary Graph recurrence editor.

### Outlook category catalogue

Enable **Settings → Connected apps → Connect Outlook categories** to request `MailboxSettings.Read` and read `/me/outlook/masterCategories`. This permission supports both personal and organizational accounts where the [endpoint is available](https://learn.microsoft.com/en-us/graph/api/outlookuser-list-mastercategories?view=graph-rest-1.0). It does not grant access to message bodies.

The detail picker shows the catalogue's names and color dots alongside the task's existing categories. Task membership remains a list of category names. The catalogue itself is read-only: create, rename, and recolor master categories in Outlook. Microsoft defines 25 [semantic color presets](https://learn.microsoft.com/en-us/graph/api/resources/outlookcategory?view=graph-rest-1.0), with exact RGB values varying by client. This app maps those presets to its Fluent palette; an unknown preset receives a neutral color.

The catalogue is cached in memory per account and cloud for five minutes. Explicit refresh bypasses the interval. Concurrent reads share one request, a failed refresh retains a prior successful catalogue, and disconnect prevents a late request from repopulating the cache. Catalogue data and tokens are separate from task backups.

### OneDrive device settings

Enable **Settings → Connected apps → Sync device settings with OneDrive** on each installation. This requests `Files.ReadWrite.AppFolder` and uses `/me/drive/special/approot`. Microsoft documents [app-folder access](https://learn.microsoft.com/en-us/graph/onedrive-sharepoint-appfolder) for both personal and work/school OneDrive. All participating installations must use the **same Microsoft application registration, account, and cloud**. The folder belongs to that app; a different client ID has a different app folder. OneDrive must be provisioned and have sufficient quota.

The versioned `ferroui.todo.presentation` format synchronizes My Day membership and per-day exclusions; local task order; list colors, background, sort, completed visibility and group membership; group names/order/collapse state; sidebar order; and selected appearance/workflow preferences. Planner importance is a personal presentation preference and is included. Task content, attachment bytes, category catalogues, credentials, client configuration, and connector/background opt-ins are excluded. This is this app's independent settings format, not access to Microsoft's private To Do My Day, group, or theme storage.

On first enable, existing remote values are adopted for fields already represented remotely; fields only present locally can be published. Each installation has a random persistent device ID and writes one `ferroui-todo-presentation-<device>.json` replica. Field-level Lamport revisions merge independent changes deterministically; simultaneous changes to one field use revision/device ordering. This does not depend on device wall-clock agreement or an unsupported shared-file atomic compare-and-swap. Group identities and pending writes are checkpointed so an interrupted upload or a local edit during network work can be reconciled on the next sync.

Unchanged replicas are checked at most once per 60 seconds; local changes and manual sync bypass that wait. Reads validate the schema, field values, remote identity/revision, and byte length before publishing a complete cache. Preauthenticated file downloads send no Graph bearer token, credentials, or referrer, reject redirects/partial responses, and accept only the selected cloud's OneDrive/SharePoint hosts. Cache keys include the account and cloud.

Limits are **4 MiB per replica**, **50,000 field records per replica**, **128 replicas**, and **16 MiB of aggregate replica bytes**. Unknown versions, invalid same-revision data, mismatched file metadata, and oversized input fail visibly without replacing the local document. Disabling the connector stops reads/writes but does not delete OneDrive data. Users can delete or replace app-folder files; this is a settings synchronization layer, not an immutable backup. Old device replicas are not automatically pruned, so the replica limit is a review boundary rather than a license to discard another device's history.

### Planner

For a connected **work or school account**, enable **Settings → Connected apps → Planner**. This also enables **Assigned to me**. Assigned tasks support title, due date, notes, checklist, and completion edits. My Day and importance are this app's presentation preferences and can travel through the optional OneDrive connection. Unsupported operations use the selected cloud's official Planner/Microsoft 365 entry point rather than an invented per-task URL.

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
| **Export Outlook PST** | A Unicode version 23 PST containing task folders, standard task fields, notes, categories, and real by-value attachments. This is a task export, not a mailbox/Purview export. |
| **Import Outlook PST…** | Read ANSI versions 14/15 or Unicode version 23 task stores; preserve source MAPI properties and restore independent local copies through the validated portable pipeline. |

The JSON archives are **application-owned formats**. PST import/export is a separate interoperable format with bounded task-field mapping; see [Outlook PST details](OUTLOOK-PST.md). Microsoft's [To Do overview](https://learn.microsoft.com/en-us/graph/todo-concept-overview) documents the relationship between To Do and PST fields. Its [announced future To Do export](https://support.microsoft.com/en-us/planner/to-do-to-planner) had no published format or complete instructions at the review date; compatibility with that future format is not claimed.

PST import retains original typed MAPI properties. HTML-only notes receive a plain-text projection; RTF-only bodies and native Outlook recurrence are retained as source data with an explicit warning rather than silently flattened into a claimed equivalent rule. Export writes standard task fields and files; steps appear in notes, while steps/My Day/links/local recurrence are additionally preserved in versioned app metadata. Outlook sees standalone occurrences, not recreated native recurring series. OST, Unicode version 36/4K stores, embedded Outlook-item attachments, and linked attachments are rejected. Browser import runs in a dedicated bounded WASM worker. Native Linux/Windows/macOS import uses an isolated child with a deadline and a 1.5 GiB cumulative Rust allocation budget: freed allocations do not replenish that budget, and it is not a measurement of resident memory. Linux/Windows additionally impose OS process memory limits. Actual desktop Outlook and native OS acceptance remain unverified.

PST export also preserves imported in-progress/waiting/deferred status, low importance, percent complete, precise source timestamps, and valid start dates where the corresponding local values remain compatible. Local completion/importance edits take precedence. Clearing a due date or moving it before a retained start date omits that incompatible start with a warning. The dedicated [PST guide](OUTLOOK-PST.md#export) describes the civil/UTC field mapping and fidelity fixtures.

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

Success sets automatic sync, Planner, Outlook categories, OneDrive settings sync, and native keep-running **off**, removes active cloud/list/provider bindings and assignment/flagged-email projection markers, disconnects this app's Microsoft session, and clears synchronization mappings. Source URLs remain as links. Restored flagged-email and Planner items become ordinary local tasks. The status reads **“Restored as a local copy. Microsoft sync is paused.”**

Importing never uploads to Microsoft. To publish the independent copies, connect the intended account and explicitly sync; this creates new tasks instead of updating original remote identities. Document replacement participates in local undo; undo does not reconnect a Microsoft session. Exported archives retain original source fields for inspection or other tools.

## Conflicts, moves, and recovery

The sidecar keeps separate local/remote baselines. Disjoint field edits merge; incompatible edits to the same field stop visibly. Conditional entity tags protect updates where available. Planner requires independent task/detail tags, as documented for [task updates](https://learn.microsoft.com/en-us/graph/api/plannertask-update?view=graph-rest-1.0) and [detail updates](https://learn.microsoft.com/en-us/graph/api/plannertaskdetails-update?view=graph-rest-1.0).

Before remote mutations and local replacement, the client checks that the captured document is current. Web Locks prevent overlapping cloud sync from tabs at the same origin. Account changes and restore/export also use the serialized queue. Browsers without Web Locks report that safe cloud sync is unavailable. The native host locks its application-data directory to one process and accepts worker document replacements only after an application-thread snapshot comparison.

Moves preserve writable source properties, steps, links, and file bytes. The destination is populated before source deletion. The source is read again, attachment content is checked, and conditional deletion rejects a changed source. A late conflict retains both copies for review. Graph supplies no atomic transaction for an entire multi-request sync or account export: earlier operations can succeed before a later failure, and exports are not server-wide transactional snapshots.

Durable creation intents, correlations, upload progress, and a commit journal recover interrupted operations. Ambiguous outcomes stop for review. Deleted local files remain recoverable through attachment trash for undo; unreferenced trash is cleaned after its retention period. For conflicts, preserve a local backup, reconcile the reported Microsoft task fields, and retry. Clearing browser storage also removes local tasks and files and is not a normal recovery step.

## Platform and verification scope

Microsoft OAuth, Graph/Planner, optional categories/OneDrive settings, attachment services, portable/Graph archives, and PST export are available through the browser host and the native worker. PST import uses the bounded browser worker or isolated native parser child on Linux, Windows, and macOS. Native task editing remains Rust/FerroUI; the worker supplies service I/O through private pipe messages. A current Node.js runtime, native public-client registration, and the platform's service facilities are prerequisites. See [native service architecture and limits](NATIVE-MICROSOFT.md).

Reminders are checked while running and on resume. Browser notifications require permission and platform support. Native notifications and tray availability depend on the OS/session; keep-running is offered only when a usable tray is present. Native background operation keeps the existing process alive; no background server, webhook receiver, or push service wakes a closed process. Live OS notification/tray and authenticated sovereign-tenant acceptance require separate testing.

Run `node --test scripts/*.test.mjs` for the host/auth/Graph/Planner/category/OneDrive/native/archive fixtures. Tests exercise production modules with mocked MSAL and Graph, including optional consent cancellation, cloud/scope isolation, bounded downloads, replica merging/recovery, delta recovery, files, moves, redirects, serialized host work, and validation before restore. PST tests write real stores and parse them with Microsoft's reader; they do not replace Outlook application acceptance. Rust tests cover document validation, domain capabilities, and shared PST projection. These checks do not establish that a particular registration, tenant consent policy, license, live account, OS service, or deployment has been verified; current CI and authenticated acceptance results must be checked separately.

Vendored authentication, PST, HTML parsing, and Unicode time-zone dependencies retain their licenses beside the corresponding browser/native assets and in the crate notices.
