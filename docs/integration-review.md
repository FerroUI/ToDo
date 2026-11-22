# Microsoft To Do integration review

Reviewed **2026-11-22** against the primary Microsoft documentation linked below and the repository implementation. The implementation baseline was `fb15bde12c68ca48cc3dad01cafde6019628cacf`. This review covers external services and data portability; it does not certify complete product or visual parity.

## Findings and changes

The baseline already had a local-first Rust application and two-way Graph task synchronization. Its Microsoft connection depended on a manually entered client ID, ran on explicit requests, stored attachments only locally, and lacked a connected Planner workflow. JSON backups contained file references instead of file bytes.

This work adds public deployment configuration, MSAL session/renewal/logout handling, foreground automatic sync with delta reads, Graph attachment transfer and recovery, source-link and category visibility, Planner assignments with supported edits, and two versioned archives containing real file bytes. Restore creates an independent local copy and pauses/disconnects cloud sync. [Setup and operation](MICROSOFT.md) documents exact behavior and limits.

The deployment still needs an owner-controlled Entra registration and public client ID. No registration was invented, no client secret is embedded, and no authenticated Microsoft account was used for live acceptance testing during this implementation.

## Original product and public API comparison

| Area | Microsoft reference | Current implementation and boundary |
| --- | --- | --- |
| Sign-in | Public browser apps use [authorization code with PKCE](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow). | MSAL Browser 5.30.0 redirects, selected-account restoration, renewal, reconnect, local disconnect, and Microsoft logout. Runtime configuration supports `common`, `organizations`, `consumers`, or tenant GUID. A real registration is still required. |
| Automatic sync | Microsoft documents [five-second To Do updates and Outlook task sync](https://support.microsoft.com/en-us/todo/syncing-your-microsoft-to-do-account). | Five-second foreground interval, 900 ms edit debounce, hidden/offline pause, manual sync, and refresh integration. List/task deltas hydrate changed children. No closed-app synchronization. |
| Lists/tasks | [Task lists](https://learn.microsoft.com/en-us/graph/api/resources/todotasklist?view=graph-rest-1.0) and [tasks](https://learn.microsoft.com/en-us/graph/api/resources/todotask?view=graph-rest-1.0) expose CRUD, details, and children. | Two-way CRUD, steps, dates, supported recurrence, conflict-aware merging, conditional writes, and preservation of unedited unsupported properties. The editor does not expose every Graph status/body/recurrence representation. |
| Files | Graph supports task attachments and [upload sessions](https://learn.microsoft.com/en-us/graph/api/taskfileattachment-createuploadsession?view=graph-rest-1.0). | Upload/download/delete, on-demand IndexedDB cache, resumable large uploads, uncertain-upload recovery, and file-preserving moves. App limit: 25 MiB per file. Offline files must already be cached. |
| Outlook/flagged email | The `flaggedEmails` task list and [To Do service integration](https://learn.microsoft.com/en-us/graph/todo-concept-overview) expose task interoperability. | Import the task projection and returned source links. No inbox, mail-body reading, or arbitrary message flagging; no mail permission requested. |
| Planner | Original To Do has [Connected apps → Planner and Assigned to me](https://support.microsoft.com/en-us/planner/see-planner-tasks-in-microsoft-to-do). | Organizational-account opt-in, assigned tasks, title/due date/notes/steps/completion edits, local My Day, and Open in Planner. Sixty-second unchanged polling; edits/manual requests bypass it. |
| Planner revisions | [Task](https://learn.microsoft.com/en-us/graph/api/plannertask-update?view=graph-rest-1.0) and [detail](https://learn.microsoft.com/en-us/graph/api/plannertaskdetails-update?view=graph-rest-1.0) updates require their own entity tags. | Separate revisions/baselines, durable checklist intents, stable identities, and conflicts. Planner projections cannot accidentally upload as To Do tasks. Personal accounts are unsupported by the [assigned-tasks API](https://learn.microsoft.com/en-us/graph/api/planneruser-list-tasks?view=graph-rest-1.0). |
| Source links/Teams | [Linked resources](https://learn.microsoft.com/en-us/graph/api/resources/linkedresource?view=graph-rest-1.0) point to the originating application. | Safe returned URLs and names are shown and preserved. To Do/Planner tasks surfaced through Teams synchronize through their task services. No arbitrary Teams chat/message importer. |
| Power Automate | Microsoft's [To Do Business connector](https://learn.microsoft.com/en-us/connectors/todo/) operates on the To Do service. | Supported changes made by external flows are visible through subsequent task sync. No flow designer, deployment, or separate automation account is embedded. |
| Categories | Task categories are names; the [master category API](https://learn.microsoft.com/en-us/graph/api/outlookuser-list-mastercategories?view=graph-rest-1.0) requires `MailboxSettings.Read`. | Category names synchronize. Master catalogue, palette, and colors remain unimplemented; the extra scope is not requested. |
| Sharing/assignment | Lists expose `isOwner`/`isShared`; reviewed public To Do task/list methods do not supply invitation, membership, or original per-task assignee workflows. | Retain sharing/ownership metadata and guard unsupported list actions. No invitations, membership management, or To Do cross-user assignment. Planner assignments use a separate service. |
| My Day/organization | Reviewed To Do v1.0 schemas do not expose My Day, list groups, or themes. | These features, local ordering, and appearance remain device-local presentation state. |
| Backup/import | Microsoft documents [PST field mapping](https://learn.microsoft.com/en-us/graph/todo-concept-overview); the announced future export has no published format yet. | Legacy document JSON, portable document-plus-files JSON, and raw Graph account archives. No PST reader/writer or unpublished-format compatibility claim. |
| Reminders/desktop | Original clients use their own platform and service infrastructure. | Local reminders/browser notifications while running or resuming. Native desktop cloud integration, background push, and closed-app reminders remain unimplemented. |

## Architecture and data integrity

The Rust document and MVVM state remain authoritative. Browser service modules live under `crates/todo-browser/wwwroot/cloud/`; archive validation and file staging live under `wwwroot/backup/`. Provider identities and synchronization history are separate from user-facing task fields. Planner projections carry explicit external provenance, and domain actions guard operations that the integration does not support.

One host queue serializes Microsoft work. Account changes and restore/export pause scheduled work; Web Locks coordinate tabs. To Do uses three-way merging and conditional updates. Planner uses independent task/detail revisions and recoverable checklist writes. The complete sync spans multiple requests, so it cannot be atomic. A changed source discovered during a move leaves both task copies available for review.

Files travel with moves: download source content, record checksums, populate the destination, then recheck the source before deletion. Recovery metadata distinguishes previously uploaded content from local additions. Account-specific cache keys and document bindings prevent another account from inheriting pending writes.

Foreground scheduling limits idle work, backs off transient failures, stops on authentication/service conflicts, and permits Planner to pause independently. Delta snapshots are committed only after complete reads; invalidated cursors trigger a full fetch. These are implementation decisions, not guarantees about network latency or Microsoft's availability.

### Portable restoration

| Archive kind/version | Payload | Restore behavior |
| --- | --- | --- |
| `ferroui.todo.backup` / 1 | Canonical local document, unique file bytes, metadata, opaque file keys, SHA-256. | Validate all content, allocate fresh file keys, replace the local document once. |
| `ferroui.microsoft-todo` / 1 | Fresh Graph lists/tasks, original returned properties, hydrated children, files with source mappings/checksums, export time. | Allocate new local identities, retain original task data for restricted future-upload seeding, then use the portable restore pipeline. |

The account archive performs no remote writes. It covers accessible To Do data, excluding Planner plans/mailboxes, and is not a server-transactional snapshot. Both formats cap single files at 25 MiB, decoded file contents at 128 MiB, metadata/document JSON at 32 MiB, and archive input/output at 256 MiB.

Restore validates version, IDs, relationships, file references, sizes, base64, and checksums before storage writes. Fresh keys protect existing attachments. Guards detect concurrent local edits; failure rolls back newly staged blobs. Unsupported future versions are rejected.

Successful restoration disables auto-sync and Planner, removes active cloud/provider bindings and assignment/flagged-email markers, disconnects the local Microsoft session, and clears mappings. Source URLs survive. Restored tasks are independent local copies requiring intentional connection/sync before publication. Tokens, profile wrappers, and synchronization sidecars are not archive payloads.

## Remaining work and dependencies

| Item | Required next work | Status |
| --- | --- | --- |
| Production sign-in | Owner-controlled Entra SPA registration, exact `https://ferroui.com/ToDo/` redirect, public `MICROSOFT_CLIENT_ID` repository variable, consent, and live personal/work-account verification. | Registration/configuration and authenticated acceptance outstanding; implementation/deployment-variable path present. |
| Native service host | Native public-client OAuth callback/token storage, Graph services, lifecycle scheduling, attachment transfer. | Unimplemented; desktop Settings opens the browser app. |
| Category palette | Optional `MailboxSettings.Read` consent, catalogue fetch, colors/UI. | Public API available; only task category names implemented. |
| Background sync/push | Supported background or server execution, delivery permissions, session model, operational hosting. | Static Pages deployment has no background service. |
| Direct email/chat actions | Explicit product scope, Mail/Teams permissions, service-specific UX. | No inbox/chat access requested; existing task interoperability is implemented. |
| Original list collaboration | Documented original invitation/member/assignee contract or handoff to Microsoft's client. | No equivalent methods found in reviewed public To Do v1.0 resources. Sharing flags alone are insufficient. |
| Cloud My Day/groups/themes | Public properties/endpoints equivalent to original client state. | No equivalent fields in reviewed To Do resources; local only. |
| PST/future export import | Published actual format, parser/mapping, loss accounting, fixtures. | Unimplemented; JSON archives have explicit app-owned kinds/versions. |

The browser targets global Microsoft identity and Graph endpoints; national-cloud hosts are not configurable. A supported endpoint does not guarantee tenant consent or a Planner license.

## Service lifecycle context

As reviewed on 2026-11-22, Microsoft says its To Do applications will retire **June 30, 2027**. It distinguishes the underlying service, APIs, and synchronization, whose later retirement dates are not yet announced. Export guidance is promised before app retirement, but format details and limits are not published. Existing tasks will not automatically migrate to Planner, and Planner is unavailable to personal Microsoft accounts. These facts affect planning; they do not establish that Graph has already retired. [Microsoft's transition guidance](https://support.microsoft.com/en-us/planner/to-do-to-planner).

## Provider capabilities across input paths

Planner task restrictions apply to native menus, detail actions, swipe rails, mouse and held-touch drag, keyboard deletion, and bulk operations. Planner swipes expose My Day only; flagged-email tasks cannot move into a personal list. Single-action rails use their actual width for drag limits and settling thresholds. Rejected protected operations preserve the current selection and cannot produce a false deletion-success message. These policies remain enforced by the Rust domain beneath every UI route.

The combined-input browser review also exposed a framework lifetime bug: an already completed fallback drag could consume the next mouse press before its async cleanup ran. The FerroUI submodule includes a synchronous completed-result guard, with focused regressions for drop and Escape completion before future cleanup. App-level gesture ownership and cancellation policies remain in the existing coordinator.

## Verification and limits

The browser suite exercises production modules with controlled MSAL, storage, timer, and Graph fixtures. Focused files include `microsoft-auth*.test.mjs`, `automatic-sync.test.mjs`, `graph-sync.test.mjs`, `graph-delta.test.mjs`, `planner-sync.test.mjs`, `portable-backup.test.mjs`, and `microsoft-archive.test.mjs`. They cover redirects, token renewal, origin validation, scheduling, deltas, files, interruption recovery, move conflicts, Planner revisions, malformed archives, checksums, concurrent edits, and account isolation. Rust tests cover model constraints and document validation.

Run `node --test scripts/*.test.mjs` and the repository's current Rust/browser CI commands. Fixture tests establish implementation behavior, not a real registration, tenant policy, account license, server response, or deployed sign-in experience. Final CI/deployment results and authenticated account acceptance must be recorded separately.
