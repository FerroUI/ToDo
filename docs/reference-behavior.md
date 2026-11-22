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

- Added to My Day currently sorts current-day membership first, then manual position and ID. The public guide does not define timestamp or tie semantics; the app has no per-add timestamp and does not claim Microsoft's exact comparator.
- A task completed today stays in My Day's completed section until the next local day. This continuity policy is explicit in the domain tests; it is not presented as a documented Microsoft edge-case rule.
- Older documents without a mixed root sequence retain their prior order. The optional sequence is repaired only for missing/new navigation identities; entity relationships remain strictly validated. Ungrouping replaces the group in place with its ordered children. Sidebar order remains a local preference during Microsoft synchronization.
- Original vector art and bundled Inter remain visible differences from Microsoft's wallpapers and platform typography. Native WinUI menus improve component fidelity without asserting complete visual parity.
- Cloud sharing, Planner, server-driven suggestions, background reminder infrastructure, and complete Microsoft attachment transfer remain outside the local implementation; see [feature coverage](features.md).
