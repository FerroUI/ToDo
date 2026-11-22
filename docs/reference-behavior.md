# Microsoft To Do interaction reference

Reviewed on 22 November 2026. The shared Rust interface targets the Windows To Do composition and retains the documented web/mobile navigation model when space is limited. This ledger records the basis for changes; it does not certify exact pixels or undocumented internal behavior.

## Verified workflows

| Workflow | Microsoft primary reference | Application decision |
| --- | --- | --- |
| Navigation and details | [Explore and navigate To Do](https://support.microsoft.com/en-us/accessibility/todo/use-a-screen-reader-to-explore-and-navigate-to-do) describes a sidebar, task list, right-hand details, a Lists menu, and separate mobile navigation | Retain those surfaces; adapt their coexistence to available width |
| List creation and rename | [Work with lists](https://support.microsoft.com/en-us/accessibility/todo/use-a-screen-reader-to-work-with-lists-in-to-do) describes immediate untitled-list creation and a Rename list button that becomes an editor | Create first, edit the header inline, commit on Enter; protect the built-in Tasks list |
| Group creation and rename | [Group lists](https://support.microsoft.com/en-us/accessibility/todo/use-a-screen-reader-to-group-lists-in-to-do) describes immediate untitled groups, a focused title field, and keyboard context menus | Focus an inline group editor; expose Rename group through native context menus |
| Group membership and removal | The same group guide distinguishes moving lists, removing one member, ungrouping, and deleting an empty group; [Organize your lists](https://support.microsoft.com/en-us/todo/organize-your-lists) documents dragging lists into groups | Use explicit-ID membership/removal commands and typed sidebar drag targets; mixed root ordering is persistent; ungroup preserves every list/task in place; empty groups expose guidance and Delete group |
| Task movement | [Move tasks](https://support.microsoft.com/en-gb/todo/move-tasks-between-or-within-lists?nochrome=true) describes selection and context-menu destination choices | Build task menus from current selection and use a native destination submenu |
| Mobile task swipes | [Microsoft’s iOS tips, section 6](https://techcommunity.microsoft.com/blog/To-DoBlog/six-tips-to-get-the-most-out-of-microsoft-to-do-in-ios/2256864) and [Android tips](https://techcommunity.microsoft.com/blog/to-doblog/six-tips-to-make-the-most-of-microsoft-to-do-on-your-android-device/1933012) describe left swipe for Delete and right swipe for My Day and Move | Reveal those actions; always confirm swipe deletion. Do not map a swipe to completion |
| Touch selection/reorder | [Move tasks](https://support.microsoft.com/en-gb/todo/move-tasks-between-or-within-lists?nochrome=true) documents Android long-press selection and dragging within unsorted lists | Use FerroUI native Holding; continue moving to manually drag a task, retain native scrolling before the hold, and disable manual drag in sorted or smart views |
| Settings controls | [Microsoft support settings screenshot](https://support.microsoft.com/en-us/todo/media/get-support-windows-c3-201722718418.png) shows switch-style preferences | Compile WinUI ToggleSwitch and use its own pointer, keyboard, and knob interaction rather than emulating a switch with a checkbox |
| Sort choices | [Sort and search](https://support.microsoft.com/en-us/todo/sort-and-search-in-lists) lists importance, due date, Added to My Day, alphabetical order, and creation date; the [steps/importance guide](https://support.microsoft.com/en-us/todo/add-steps-importance-notes-tags-and-categories-to-your-tasks) illustrates a visible sort label, direction arrow, and remove button | Offer all five alongside manual order, visible reversal/removal, and persisted list direction; retain completed-task grouping and untouched manual positions |
| Scheduling | [Due dates and reminders](https://support.microsoft.com/en-us/todo/add-due-dates-and-reminders-in-microsoft-to-do) documents Today/Tomorrow/Next week/custom date, reminder quick choices, and standard/custom recurrence | Use native anchored quick menus and direct clearing; reserve a native keyboard-accessible Calendar and inline-validated form for custom values |
| Leading emoji icons | [Customize your lists](https://support.microsoft.com/en-us/todo/customize-your-lists) documents using the first emoji in a list title as its navigation icon | Resolve a complete leading emoji grapheme with FerroUI Unicode data; preserve the stored title and accessible name |
| Attachment budget | [Manage tasks with To Do in Outlook](https://support.microsoft.com/en-us/outlook/calendar/manage-tasks-with-to-do-in-outlook) and [What's new in To Do](https://support.microsoft.com/en-us/todo/what-s-new-in-microsoft-to-do) describe 25 MB of files per task | Preflight the entire browser selection and recheck aggregate bytes when committing each attachment |
| My Day due-today inclusion | [Manage tasks with My Day](https://support.microsoft.com/en-us/outlook/create-and-manage-task-lists-with-my-day-in-outlook) and the [To Do announcement](https://techcommunity.microsoft.com/blog/to-doblog/microsoft-to-do-now-supports-auto-population-of-tasks-due-today-in-the-my-day-li/3690551) describe automatic due-today inclusion | Preserve automatic inclusion and explicit removal; do not revive tasks completed before today |

## Adaptive policy

Microsoft's public guides confirm the navigation model but do not publish numerical desktop/web breakpoints or an exhaustive narrow-window specification. These are fit-based application choices:

| Available width | No right pane | Details, Settings, or Suggestions open |
| --- | --- | --- |
| Below 760px | Full-width list; navigation drawer on request | Full-width right pane; Close/Escape returns to the list and its navigation menu |
| 760–979px | 280px inline navigation and flexible list, unless manually hidden | Flexible list and 340px right pane; temporary navigation drawer |
| At least 980px | Inline navigation and flexible list, unless manually hidden | 280px navigation + at least 360px list + 340px right pane |

The drawer does not shrink the list beneath it. Manual sidebar preference survives resizing. Dialogs use available-width constraints and margins, including at 320px. Search and new-step drafts survive pane transitions; delayed desktop search cannot hide the compact search surface after a resize.

No bottom tab bar, mobile-only feature fork, or invented fixed navigation destination was introduced. Device keyboard behavior and every assistive technology remain separate validation work.

## Deliberate limits and inferences

- Public references do not specify pull-refresh or edge-Back thresholds, animation physics, or an exhaustive gesture matrix. The implementation uses an 8px direction-lock slop, a 24px left-edge zone, a 72px Back/drawer threshold, and an 80px refresh threshold as fit-based choices. Refresh retains the mounted document and drafts and uses existing explicit synchronization when connected. Browser Back uses one same-document guard and bounded in-memory view history; these details are implementation decisions, not claims about Microsoft internals.
- Touch gestures are single-contact. Multi-touch, cancellation, wheel input, Escape, route changes, and removed controls cancel an active operation without an unintended drop. Pinch zoom, device edge arbitration, screen-reader exploration gestures, and native keyboard behavior require physical-device validation.
- Added to My Day currently sorts current-day membership first, then manual position and ID. The public guide does not define timestamp or tie semantics; the app has no per-add timestamp and does not claim Microsoft's exact comparator.
- A task completed today stays in My Day's completed section until the next local day. This continuity policy is explicit in the domain tests; it is not presented as a documented Microsoft edge-case rule.
- Older documents without a mixed root sequence retain their prior order. The optional sequence is repaired only for missing/new navigation identities; entity relationships remain strictly validated. Ungrouping replaces the group in place with its ordered children. Sidebar order remains a local preference during Microsoft synchronization.
- Original vector art and bundled Inter remain visible differences from Microsoft's wallpapers and platform typography. Native WinUI menus improve component fidelity without asserting complete visual parity.
- Cloud sharing, Planner, server-driven suggestions, background reminder infrastructure, and complete Microsoft attachment transfer remain outside the local implementation; see [feature coverage](features.md).
