# Microsoft account synchronization

The browser application has an optional Microsoft Graph connection. Local tasks work immediately without an account. The Microsoft client ID is deliberately empty in a new installation: this repository does not own a Microsoft application registration or ship credentials.

## Register the browser application

1. In Microsoft Entra **App registrations**, register an application. To support both work/school and personal Microsoft accounts, choose the corresponding multitenant plus personal-account option.
2. Add a **Single-page application** platform and register the exact deployed URL, including its case and trailing slash. For this repository the expected URL is `https://ferroui.github.io/ToDo/`. A custom domain or local development origin needs its own redirect URI.
3. Add delegated Microsoft Graph permissions **Tasks.ReadWrite** and **User.Read**. Tenant consent policies may require an administrator's consent.
4. Copy the public **Application (client) ID** into the app's Settings, then select **Connect Microsoft account**. Do not create or paste a client secret into this static application.
5. After Microsoft redirects back, the app verifies the account through Graph. Select **Sync** to synchronize. A backup import never sets the account connection status.

The authentication implementation is Microsoft's vendored MSAL Browser 5.30.0. It uses authorization-code flow with PKCE, Microsoft-managed state and nonce validation, and `sessionStorage` token caching. Tokens are neither included in task exports nor stored in task documents. Sign-in is a full-page redirect, and the registered redirect URL serves the application itself. Refresh-token renewal runs through MSAL; a session that needs interaction asks the user to reconnect instead of opening an invisible sign-in iframe. Disconnect clears this application's session; it does not sign the user out of other Microsoft applications.

## Data synchronization

Synchronization is explicit and two-way. The implementation uses the supported Graph v1.0 task-list, task, checklist-item, and linked-resource endpoints. It reads all result pages and sends only changed fields.

| Data | Behavior |
| --- | --- |
| User lists | Create, rename, import, and delete owned lists. Preserve Microsoft's built-in Tasks and Flagged Email lists. |
| Tasks | Create, edit, complete/reopen, move, import, and delete. A move creates the destination task before deleting the source. |
| Task details | Synchronize title, plain-text notes, importance, due date, reminder, categories, and supported repeating schedules. Notes are safely encoded as Graph HTML bodies. |
| Steps | Synchronize checklist creation, rename, completion, and deletion, preserving local identifiers. |
| Flagged email | Import the Graph flagged-email list into the Flagged Email smart view. This does not implement Outlook message flagging. |
| Local presentation | Keep My Day selections, hidden suggestions, list themes, groups, local ordering, appearance settings, and local assignment labels on this device. |
| File attachments | Keep blobs in this browser's IndexedDB. They are not uploaded to Graph. JSON backups include attachment metadata, not blob contents. |

Daily, weekly, every-weekday, monthly, and yearly schedules can be written. Graph recurrence patterns outside the local model, including relative-monthly rules, bounded recurrence ranges, and schedules whose pattern day differs from their range-start anchor, remain on the server when unrelated fields change. Custom intervals between business days cannot be written to Graph. Editing an unsupported server recurrence from this client is not a complete recurrence editor. Moving a task preserves cloud-only task properties and linked resources; tasks with Microsoft file attachments must be moved in Microsoft To Do so their files cannot be lost.

There is no background server or push service. Local reminders are checked while the application is running and when it regains focus. Browser notifications require permission and platform support; they cannot wake a closed application. Files removed from tasks remain in this browser's IndexedDB so undo can restore their references. Unreferenced files are not automatically removed by the task-editing flow.

## Conflict and interruption handling

The synchronization sidecar records separate local and remote baselines. Changes to different fields merge; simultaneous, incompatible edits to the same field stop with a visible conflict message. Conditional requests use Graph entity tags when supplied. Deleting a list checks for remote additions or changes again before deletion. The Graph API does not provide an atomic transaction across an entire synchronization, so successfully applied independent operations can precede a later conflict.

Before each remote mutation and before replacing the local document, the client checks that neither this tab nor another tab has changed the captured document. A Web Lock prevents simultaneous synchronization attempts from this origin. A browser without Web Locks reports that safe synchronization is unavailable.

Progress is checkpointed. Linked-resource correlation recovers task creation after an uncertain network response; checklist and list creation use conservative recovery. Ambiguous results stop rather than create another copy. A commit journal recovers a completed document write if saving its mapping was interrupted. An existing local document remains bound to its original Microsoft account so switching accounts cannot silently upload another person's tasks.

For a conflict, export a local backup, inspect the corresponding task in Microsoft To Do, and make the conflicting fields agree before synchronizing again. Do not clear site storage as a routine recovery step: it also contains local task data and files.

## Verification and scope

`node --test scripts/graph-sync.test.mjs` exercises the real synchronization engine against an in-memory Graph fixture. It covers CRUD, three-way merges, deletions, checklist edits, interrupted creates, task moves, local edits during synchronization, account isolation, and late remote changes. Rust tests separately validate imported documents and persisted state replacement. These tests do not replace an authenticated end-to-end check with the deployment's own Microsoft registration and account.

Shared-list invitations, cross-user assignment services, Planner integration, Outlook message APIs, Graph file uploads, and delta-based/background synchronization are not implemented by this connection.

## Primary references

- [Microsoft identity platform authorization-code flow and SPA redirect URIs](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow)
- [MSAL Browser](https://github.com/AzureAD/microsoft-authentication-library-for-js/tree/dev/lib/msal-browser)
- [Microsoft Graph To Do task resource](https://learn.microsoft.com/en-us/graph/api/resources/todotask?view=graph-rest-1.0)
- [Microsoft Graph task-list resource](https://learn.microsoft.com/en-us/graph/api/resources/todotasklist?view=graph-rest-1.0)
- [Update a task](https://learn.microsoft.com/en-us/graph/api/todotask-update?view=graph-rest-1.0)
- [Checklist-item resource](https://learn.microsoft.com/en-us/graph/api/resources/checklistitem?view=graph-rest-1.0)

Microsoft authentication code and Unicode time-zone mapping retain their licenses under `crates/todo-browser/wwwroot/vendor/`.
