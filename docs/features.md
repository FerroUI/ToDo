# Feature coverage

This document distinguishes implemented behavior from reference-app parity. The visual target is the Windows Microsoft To Do layout. The application uses original vector artwork, Inter typography, FerroUI controls, and the framework's WinUI 3 theme. Exact pixel equivalence, complete Microsoft service integration, and comprehensive device certification have not been demonstrated.

## Local task workflows

| Area | Implemented | Remaining distinction |
| --- | --- | --- |
| Smart views | My Day, Important, Planned, All, Completed; optional Assigned to me and Flagged email navigation | Connected work accounts can import assigned Planner tasks; Flagged email projects the service list returned by Microsoft Graph |
| My Day | Add/remove, local-date rollover, due-today inclusion, suggestions | Suggestion ranking is local and deterministic; Microsoft's personalization service is not reproduced |
| Tasks | Add, rename, complete/reopen, importance, delete confirmation, duplicate, move between lists | Reference-app service behaviors are covered separately below |
| Smart task entry | English trailing dates and times, relative minutes/hours, live schedule preview, settings for recognition and title retention | No arbitrary embedded expressions, free-form month dates, ranges, or natural-language recurrence; detail editing remains literal |
| Steps | Add with Enter or the add button, rename, complete, native step menus, delete, progress summary, promote to a task in the same list | Promotion preserves the step's completion state and removes the source step in one undoable command |
| Notes and search | Multiline notes, Ctrl/Command+Enter save; case-insensitive search across titles, notes, steps, and tags; clear/close returns to the previous list; accurate completed-result count | Notes are plain text; category names have an individual add/remove editor, including literal commas and leading #. No rich-text editor, inline hashtag navigation, or Outlook master-category color management |
| Dates | Native anchored due/reminder quick choices, direct remove controls, WinUI Calendar with day/month/year selection and keyboard navigation, local reminder date/time with inline validation | Reminder scheduling is active only while the application is open |
| Repeat | Daily, weekdays, weekly, monthly, yearly, custom intervals including weekdays; month-end and leap-year handling | Arbitrary weekly day sets and end-after-count/date patterns are not exposed |
| Lists | Immediate creation with inline naming; full-width title rename; duplicate, delete, color, sort, completed visibility; drag and context-menu ordering; leading emoji icons | Custom list imagery and every native context command are not reproduced |
| Groups | Immediate creation and inline rename; collapse; empty-group guidance/deletion; preserving ungroup; native membership/removal menus; drag lists into/out of groups; reorder lists and groups in a mixed root sequence | Single-entry mouse/touch dragging; a stationary touch hold opens the native menu, while movement after the hold organizes the entry |
| Task order | Manual move up/down; importance, due date, My Day membership, alphabetic, creation-date sort; visible reverse/reset controls; persistent list sort direction; mouse drag and touch hold-drag to reorder a manual list or move to a sidebar list | Added to My Day uses membership then manual position/ID; Microsoft's undocumented timestamp/tie behavior is not claimed. Single-task dragging; reorder stays within a completion group and manual List view |
| History | Undo/redo for domain edits, including task/list deletion and import | History is session-local and bounded; cloud replacement clears stale local history |
| Multi-selection | Native touch hold or explicit selection mode, Ctrl/Command-click toggles, Shift-click ranges, select-all-visible, bulk completion, move, and delete with confirmation | Each bulk edit is one undo step; drag selection and every reference-app bulk operation are not reproduced |
| Transfer | Lightweight document JSON, portable backups containing files and SHA-256 checksums, raw Microsoft To Do Graph archives, and print view | Microsoft archives are this app’s documented format, not PST or an unpublished Microsoft export format. Restore creates a disconnected local copy |
| Sharing | Export a copy; linked Microsoft To Do entry for service-managed invitations; imported owner/shared-list capability guards | Invitation creation, member/assignee administration, and collaboration presence are not exposed through the public To Do API used here |

## Appearance and interaction

| Area | Coverage |
| --- | --- |
| Desktop composition | Navigation sidebar, colored list headers, task rows, task detail pane, settings, suggestions, and modal dialogs |
| Themes | Light, dark, and system modes; list and smart-view colors |
| Controls | FerroUI WinUI theme with app-specific templates for action buttons and task rows; native Calendar, ToggleSwitch, MenuItem and ComboBox controls; real text inputs, list virtualization, scrolling, and focus |
| Context menus | Native WinUI ContextMenu/MenuItem templates, anchored pointer/keyboard opening, destination submenus, destructive colors, light dismiss, live multi-selection, explicit sidebar identity, and focus return |
| Artwork | Original vector icons and My Day landscape; not Microsoft's proprietary wallpaper catalog |
| Responsive layout | Below 760px: list or right pane with navigation drawer. With a right pane at 760–979px: list/details and temporary navigation. From 980px: 280px navigation + at least 360px list + 340px details. Sidebar preference survives resize; exact thresholds are implementation choices |
| Accessibility | Named controls and enabled FerroUI browser accessibility bridge; full assistive-technology certification remains unverified |
| Typography | Bundled Inter plus complete licensed Noto COLRv1 emoji fallback through FerroUI; the browser loads the emoji font as a separate integrity-pinned offline asset. Platform font rendering still differs from Microsoft To Do |
| Editing | Transient inline naming commits once on Enter/blur and cancels on Escape; step drafts survive pane changes; focused account/title/note/step/name edits flush before host departure; desktop search debounce rechecks compact mode; panes and menus restore focus |
| Task-list navigation | Arrow keys, Home/End, and Page Up/Down move the task cursor and scroll it into view; Shift extends selection; controls expose action names |
| Sidebar navigation | Up/Down/Home/End traverse visible entries, Left/Right operate the group hierarchy, Space expands/collapses, F2 renames, and menus provide keyboard alternatives to dragging |
| Touch task actions | Swipe left to reveal Delete with confirmation; swipe right for Add/remove My Day and Move; vertical scrolling, native Holding selection, manual hold-drag, cancellation, and autoscroll share FerroUI pointer capture; custom lists/groups support hold-and-drag organization and stationary-hold menus. Service capabilities limit Planner to the My Day swipe action and omit Move for flagged email |
| Back and refresh | Browser history Back, native host Back requests exposed by FerroUI, and Alt+Left first cancel a pending/active gesture, then use the shared active-surface/prior-list navigation path; left-edge Back and left-swipe drawer dismissal; signed pull progress at the list top refreshes the local clock and shares an in-flight account sync without reloading or discarding drafts |
| Gesture combinations | Stable contact quarantine, reversal/axis lock, terminal drag origins, native-scroll release guards, mixed mouse/touch interruption, resize/deactivation cleanup, and trusted fresh-touch recovery after a missed release; mouse dragging retains wheel scrolling |
| Status and typography | Reserved status footer with relevant Undo, wrapping long actions, bounded metadata and two-line task titles, 14px body editors/actions, 12px supporting labels, and native 48px Settings switch rows |
| Header and search | Full-width titles, selection in More, visible sort status/direction/reset, clear search, and a compact close-search affordance |

The application implements keyboard commands while the app has focus. Browser-reserved shortcuts can still be intercepted by the browser; use Ctrl/Command + Alt + N to focus the task composer in a browser tab. Ctrl/Command + N remains available in the desktop app.

| Shortcut | Action |
| --- | --- |
| Enter in the add field | Add task or step |
| Ctrl/Command + N; Ctrl/Command + Alt + N in a browser tab | Focus new task |
| Ctrl/Command + L | New list |
| Ctrl/Command + F | Search |
| Ctrl/Command + D | Complete selected tasks, or toggle the current task's completion |
| Ctrl/Command + T | Add selected task to My Day |
| Ctrl/Command + Z | Undo |
| Ctrl/Command + Shift + Z or Y | Redo |
| Ctrl/Command + 1 | Toggle sidebar |
| Ctrl/Command + P | Print list |
| Ctrl/Command + R | Refresh in place; synchronize the Microsoft account when connected |
| Alt + Left | Dismiss the current surface or return to a valid previous list |
| Ctrl/Command + A, outside text editing | Select all visible tasks |
| Arrow keys / Home / End / Page Up / Page Down in the task list | Move the task cursor; hold Shift to extend the range |
| Space in the task list | Toggle completion; in selection mode, toggle selection |
| Enter in the task list | Open task details; in selection mode, toggle selection |
| Shift + F10 or the context-menu key | Open the focused task, list, or group menu |
| F2 in navigation | Rename the focused custom list or group |
| Arrow keys / Home / End in navigation | Move between visible entries; Left/Right navigates a group and its members |
| Arrow keys / Page Up / Page Down in Calendar | Move dates or months; Enter selects a date |
| Ctrl/Command + Enter in notes | Save the current multiline note |
| Escape | Cancel inline naming, dismiss a menu/drawer, close a panel/details, or exit selection |
| Delete, outside text editing | Delete the selected tasks or current task |

## Platform coverage

| Capability | Browser | Desktop host |
| --- | --- | --- |
| Shared Rust UI and local task model | Implemented | Implemented |
| Persistence | `localStorage`, with visible write failures | Application-data JSON using temporary write and rename |
| Attachments | Add/download/remove; IndexedDB blobs; 25 MB per file with whole-selection preflight; Microsoft file transfer, resumable uploads and guarded cross-list moves | Not implemented; the host reports that the browser is required |
| Backup and restore | Document JSON, portable backups with files, Microsoft Graph archives; validated restoration with file rollback | Native save/open dialogs for document JSON |
| Printing | Browser print window | Generated HTML opened with the platform launcher |
| Completion sound | Synthesized through Web Audio, subject to browser audio policies | No audio implementation |
| Reminders | Browser notifications with permission while app is open | In-app status messages while app is open |
| Microsoft sign-in and sync | MSAL code flow with PKCE; automatic foreground/manual Graph sync; optional work-account Planner integration | Not implemented |
| Back navigation | Same-document browser history integration, covered by real browser input | Native TopLevel Back requests forward to shared dismissal and edit-flush handling; platform runtime verification remains outstanding |
| Offline installation | Manifest, immutable version-addressed runtime files, integrity-checked offline installation, safe navigation fallback, and scoped preview bypass | Normal native executable |
| Packaging | GitHub Pages site | Source host; no signed installers or app-store packages |

Browser data belongs to the current origin/profile. Clearing site data removes local documents and attachments. Connected Microsoft tasks transfer attachments through Graph; local-only files can travel in portable backups. A closed page or installed app is not guaranteed to deliver reminders; background push infrastructure is not included.

Gesture validation uses real CDP touch contacts and mouse/touch combinations in Chromium. The Rust recognizers also accept pen input, but stylus and mixed pen/finger behavior have not been verified. At the pinned FerroUI revision, the browser bridge forwards pointer cancellation for touch only, and the app's multiple-contact quarantine tracks touch contacts. Pen cancellation, mixed pen/finger arbitration, physical-device edge gestures, and native-host gestures are therefore outside the demonstrated coverage.

## Microsoft Graph synchronization

The browser integration requires an Entra SPA client ID, an exact registered redirect URI, and consent for delegated `Tasks.ReadWrite` and `User.Read`. Public deployment configuration supplies the client ID for ordinary sign-in; Settings retains a local override. The default local mode does not contact Microsoft Graph. See [account setup](MICROSOFT.md) and the detailed [integration review](integration-review.md).

| Area | Implemented or bounded behavior |
| --- | --- |
| Authentication | Bundled MSAL, authorization code flow with PKCE, redirect sign-in, silent acquisition, forced refresh after 401, reconnect, local disconnect, Microsoft sign-out, and session-only tokens |
| Automatic synchronization | One foreground timer, 5-second To Do polling, 900 ms local-save debounce, visibility/offline pause, bounded retry backoff, and explicit retry for conflicts/permissions |
| Incremental reads | List/task delta cursors with deletion handling, pagination validation, cache invalidation, and full-read fallback after an expired cursor; cursors are memory-only |
| Lists | Default/custom list mapping, create/import/rename, guarded deletion, owner/shared flags, protected service-list capabilities |
| Tasks | Create/import/update/delete and move through mapped lists, checkpointed identities, and safe copy-before-delete moves including files |
| Fields | Title, plain-text projection of notes, importance, completion, due date, reminders, supported recurrence, categories/tags; unsupported original Graph fields survive archive restoration as a whitelisted upload seed |
| Checklist items | Import, create, update, and delete steps with remote/local identity mapping |
| Attachments | Import metadata, on-demand download/cache, create and delete files, upload sessions up to 25 MB per file, SHA-256 recovery after an uncertain upload, and interruption-safe attachment moves |
| Linked resources | Preserve source metadata and display HTTP(S) source actions; internal retry markers are not shown as user sources |
| Planner | Opt-in work-account Assigned to me projection; title, completion, due date, notes, and checklist edits; separate task/detail ETags and checkpoints; 60-second unchanged polling with edits/manual sync bypassing the delay |
| Outlook/Flagged email | Import the flagged-email list returned by Graph, preserve source links, and interoperate with tasks stored in the same Microsoft To Do service; no arbitrary mailbox search or message flagging |
| Conflicts and concurrent editing | Per-field baselines, conditional writes, stale-document replacement guards, preserved local edits, and Web Locks for same-origin tabs |
| Interrupted writes | Durable mappings, creation markers, upload recovery, and a document/mapping commit journal; no blind replay of ambiguous create operations |
| Account changes and restore | One serialized account/sync/export/restore queue; a document cannot silently upload to a different account; restored copies clear old service mappings and disable automatic sync |
| Archives | Portable document/file backups and raw To Do Graph/file exports; strict validation, checksums, fresh local storage keys, and rollback of staged files on failed restore |
| Local-only data | My Day, manual positions, group topology, visual preferences, and general shared-list assignment editing remain outside public To Do synchronization |
| Recurrence limits | Supported non-ending daily/weekly/monthly/yearly patterns and ordinary weekdays; unsupported server patterns are preserved remotely unless explicitly changed |
| Service gaps | Invitation/member management, complete assignment workflow, Outlook master-category colors, application-owned push/background services, PST tooling, native-host authentication, and Microsoft private metadata |

Planner tasks retain their external identity and are excluded from ordinary To Do task creation/deletion. The UI hides unsupported move/delete/reminder/recurrence/file/category commands. Swipe rails use only the actions available to the source task, mouse/held dragging rejects protected sources, and the domain rejects unsupported actions even through batches. Keyboard deletion and mixed selections retain the task/selection and cannot report a successful deletion after a rejected operation. Project-derived Planner tasks may impose further service restrictions; failed writes keep the local edit and report the restriction. My Day and importance remain local preferences for these projections.

Automated tests exercise simulated Graph and authentication services, including failed requests, expired delta cursors, large-file upload sessions, account changes, restored file bytes, Planner conflicts, and concurrent host requests. Live sign-in, consent, tenant-specific behavior, real cross-device conflicts, and every recurrence/time-zone combination remain unverified. A sync can apply remote writes before a later request fails; the next attempt uses saved checkpoints. It is not an all-or-nothing transaction across both stores.

## Validation scope

Rust tests across `todo-domain` and `todo-state` cover date/recurrence edge cases, My Day behavior, relationships, import validation, reactive cache changes, reentrant callbacks, undo/redo, persistence failures, and reminder deduplication. Selection regression tests cover atomic bulk edits, recurrence during bulk completion, deletion confirmation, range anchors, navigation resets, and completed-row visibility. The JavaScript Graph tests cover the adapter using a simulated service.

The repository includes one browser editing and rendering gate covering task/list creation, inline naming and cancellation, native menus and submenus, live multi-selection, date recognition, focused-button activation, pane/menu/modal focus restoration, step drafts and promotion, drag/move/reorder/cancellation, retained title editing, and focused-note/account persistence on reload. It checks six viewport widths and captures light/dark/desktop/intermediate/narrow/menu/Calendar/Settings views. Real CDP touch contacts exercise action rails, native hold selection, scrolling, drag across source recycling, second-contact cancellation, list/group movement, Back/drawer navigation, and refresh; pure state tests cover gesture arbitration and single-flight refresh. Browser history tests verify same-document entry ownership and startup/reload behavior. This check does not establish pixel equivalence, complete accessibility, mobile keyboard fidelity, large-data frame times, native platform certification, or live Microsoft integration. The [reference ledger](reference-behavior.md) records supported interaction decisions and their limits.
