# Feature coverage

This document distinguishes implemented behavior from reference-app parity. The visual target is the Windows Microsoft To Do layout. The application uses original vector artwork, Inter typography, FerroUI controls, and the framework's WinUI 3 theme. Exact pixel equivalence, complete Microsoft service integration, and comprehensive device certification have not been demonstrated.

## Local task workflows

| Area | Implemented | Remaining distinction |
| --- | --- | --- |
| Smart views | My Day, Important, Planned, All, Completed; optional Assigned to me and Flagged email navigation | Assigned/flagged data depends on available model or cloud data; this is not complete Planner or Outlook integration |
| My Day | Add/remove, local-date rollover, due-today inclusion, suggestions | Suggestion ranking is local and deterministic; Microsoft's personalization service is not reproduced |
| Tasks | Add, rename, complete/reopen, importance, delete confirmation, duplicate, move between lists | No complete natural-language task-entry parser |
| Steps | Add, rename, complete, delete, progress summary | Domain supports promotion to a task; no dedicated promotion UI is exposed |
| Notes and search | Multiline notes; case-insensitive search across titles, notes, steps, and tags | Notes are plain text; no complete rich-text editor or dedicated tag-management interface |
| Dates | Due-date shortcuts and calendar selection, local reminder date/time | Reminder scheduling is active only while the application is open |
| Repeat | Daily, weekdays, weekly, monthly, yearly, custom intervals; month-end and leap-year handling | Custom business-day intervals are supported by the domain but not the custom-repeat UI |
| Lists | Create, rename, duplicate, delete, color, sort, completed visibility | Full drag-and-drop list ordering is not exposed |
| Groups | Create, rename, collapse, delete, move lists into/out of groups | No complete drag-and-drop group ordering |
| Task order | Manual move up/down; importance, due date, alphabetic, creation-date sort | Manual movement uses menu commands rather than drag-and-drop gestures |
| History | Undo/redo for domain edits, including task/list deletion and import | History is session-local and bounded; cloud replacement clears stale local history |
| Multi-selection | Selection mode, Ctrl/Command-click toggles, Shift-click ranges, select-all-visible, bulk completion, move, and delete with confirmation | Each bulk edit is one undo step; drag selection and every reference-app bulk operation are not reproduced |
| Transfer | JSON export/import and print view | JSON includes attachment references, not file bytes |
| Sharing | Export a copy; link to Microsoft To Do for service-managed invitations | Live invitation creation, access management, collaboration presence, and shared-list permissions are not implemented locally |

## Appearance and interaction

| Area | Coverage |
| --- | --- |
| Desktop composition | Navigation sidebar, colored list headers, task rows, task detail pane, settings, suggestions, and modal dialogs |
| Themes | Light, dark, and system modes; list and smart-view colors |
| Controls | FerroUI WinUI theme with app-specific templates for action buttons and task rows; real text inputs, list virtualization, scrolling, and focus |
| Artwork | Original vector icons and My Day landscape; not Microsoft's proprietary wallpaper catalog |
| Responsive layout | Separate full-width panes below 760 pixels; multi-column layout above that width |
| Accessibility | Named controls and enabled FerroUI browser accessibility bridge; full assistive-technology certification remains unverified |
| Typography | Bundled Inter through FerroUI; therefore not identical to every platform's Microsoft To Do font rendering |
| Editing | Task title, notes, and steps commit on their implemented focus/keyboard paths; search is debounced on desktop |
| Task-list navigation | Arrow keys, Home/End, and Page Up/Down move the task cursor and scroll it into view; Shift extends selection; controls expose action names |

The application implements keyboard commands while the app has focus. Browser-reserved shortcuts can still be intercepted by the browser.

| Shortcut | Action |
| --- | --- |
| Enter in the add field | Add task or step |
| Ctrl/Command + N | Focus new task |
| Ctrl/Command + L | New list |
| Ctrl/Command + F | Search |
| Ctrl/Command + D | Complete selected tasks, or toggle the current task's completion |
| Ctrl/Command + T | Add selected task to My Day |
| Ctrl/Command + Z | Undo |
| Ctrl/Command + Shift + Z or Y | Redo |
| Ctrl/Command + 1 | Toggle sidebar |
| Ctrl/Command + P | Print list |
| Ctrl/Command + R, when connected | Synchronize Microsoft account |
| Ctrl/Command + A, outside text editing | Select all visible tasks |
| Arrow keys / Home / End / Page Up / Page Down in the task list | Move the task cursor; hold Shift to extend the range |
| Space in the task list | Toggle completion; in selection mode, toggle selection |
| Enter in the task list | Open task details; in selection mode, toggle selection |
| Shift + F10 or the context-menu key | Open the current task's menu |
| Escape | Close the active panel/details, exit selection, or close the narrow sidebar |
| Delete, outside text editing | Delete the selected tasks or current task |

## Platform coverage

| Capability | Browser | Desktop host |
| --- | --- | --- |
| Shared Rust UI and local task model | Implemented | Implemented |
| Persistence | `localStorage`, with visible write failures | Application-data JSON using temporary write and rename |
| Attachments | Add/download/remove metadata; IndexedDB blobs; 25 MB per file | Not implemented; the host reports that the browser is required |
| Backup and restore | Download/upload JSON | Native save/open dialogs |
| Printing | Browser print window | Generated HTML opened with the platform launcher |
| Completion sound | Synthesized through Web Audio, subject to browser audio policies | No audio implementation |
| Reminders | Browser notifications with permission while app is open | In-app status messages while app is open |
| Microsoft sign-in and sync | Optional MSAL and Graph integration | Not implemented |
| Offline installation | Manifest and content-versioned service worker after resources are cached | Normal native executable |
| Packaging | GitHub Pages site | Source host; no signed installers or app-store packages |

Browser data belongs to the current origin/profile. Clearing site data removes local documents and attachments. The app does not provide automatic cross-device attachment transfer. A closed page or installed app is not guaranteed to deliver reminders; background push infrastructure is not included.

## Microsoft Graph synchronization

The implementation requires an Entra SPA client ID and an account that can consent to the requested delegated permissions. Connecting an account enables explicit **Sync now**. The app's default local mode does not contact Microsoft Graph.

| Area | Implemented or bounded behavior |
| --- | --- |
| Authentication | Bundled MSAL, redirect sign-in, PKCE, silent token acquisition, session storage, disconnect |
| Lists | Map default/custom lists, create/import/rename, guarded deletion |
| Tasks | Create/import/update/delete and move through mapped lists, with checkpointed identities |
| Fields | Title, plain-text projection of notes, importance, completion, due date, reminders, supported recurrence, categories/tags |
| Checklist items | Import, create, update, and delete steps with remote/local identity mapping |
| Conflict handling | Per-field baselines; conflicting edits stop synchronization; ETags used where available |
| Concurrent local edits | Export/compare check rejects a stale replacement; current local data is retained |
| Multiple tabs | Web Locks serialize sync operations |
| Interrupted writes | Checkpoints and linked resources support retry without blindly duplicating tasks |
| Account isolation | A document already associated with one account is not silently uploaded to another |
| Flagged email | Can project a Graph flagged-email list when returned by the service; does not implement Outlook's complete mail workflow |
| Local-only data | My Day, manual positions, group topology, visual preferences, attachment blobs, and assignment metadata are not fully synchronized |
| Recurrence limits | Supported non-ending daily/weekly/monthly/yearly patterns and ordinary weekdays; unsupported server patterns are preserved remotely unless explicitly changed |
| Service gaps | No Planner integration, complete assignment workflow, invitation/permission administration, push synchronization, or Microsoft attachment transfer |

Automated tests exercise a simulated Graph service. Live sign-in, consent, account-specific Graph behavior, real conflicts across devices, and synchronization of every supported recurrence/time-zone combination remain unverified. A sync can apply some remote writes before a later request fails; the next attempt uses saved checkpoints. It is not an all-or-nothing transaction across both stores.

## Validation scope

Rust tests across `todo-domain` and `todo-state` cover date/recurrence edge cases, My Day behavior, relationships, import validation, reactive cache changes, reentrant callbacks, undo/redo, persistence failures, and reminder deduplication. Selection regression tests cover atomic bulk edits, recurrence during bulk completion, deletion confirmation, range anchors, navigation resets, and completed-row visibility. The JavaScript Graph tests cover the adapter using a simulated service.

The repository includes a browser rendering smoke gate. This document makes no claim that a single smoke check establishes visual parity, complete accessibility, mobile keyboard fidelity, large-data frame times, native platform certification, or live Microsoft integration.
