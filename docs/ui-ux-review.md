# Microsoft To Do comparison and polishing review

Reviewed 22 November 2026 against the public Microsoft references in [the interaction ledger](reference-behavior.md), the deployed application, the Rust implementation, and the browser host. The starting application revision was `257a169263ac77d8220a62cf098d0368e2b13abc`.

The target remains the Windows To Do composition: navigation at the left, a themed list, optional details at the right, and the same destinations presented separately when space is limited. The documentation and screenshots establish workflows and visual hierarchy. They do not establish Microsoft's current private implementation, exact breakpoints, pixel metrics across every release, or authenticated service behavior.

## Findings addressed in this round

| Surface | Finding | Change |
| --- | --- | --- |
| Header | Toolbar width also constrained the title, truncating names unnecessarily; selection had a prominent extra icon | Separate toolbar/title rows, let the title span the full content width, and keep Select tasks in More |
| Sorting | The active sort was visible only inside its menu; there was no direct reverse control | Add the reference's visible sort status, direction action, and remove action; persist direction per list and include it in undo/redo |
| Search | Clearing/closing search did not restore its originating list; completed results could make the count disagree with the results | Add clear/close/Escape return, remember a valid origin, cancel pending debounce work, and count visible completed matches |
| Empty views | An empty connected account still received the instruction to connect; blank search displayed all tasks | Distinguish connected empty views and show a search prompt until a term is entered |
| List organization | Root lists and groups occupied separate ordering domains; drag-to-group was missing | Add a backward-compatible mixed root sequence and reuse the existing pointer gesture engine with typed task/list/group payloads |
| Group behavior | Ungrouping could collide with unrelated list positions; empty groups provided no guidance | Replace a group in place with its ordered children, show an empty drop target, and expose explicit remove/membership actions |
| Navigation access | Pointer menus existed but the sidebar lacked tree-style keyboard traversal | Add visible-entry traversal, group Left/Right behavior, F2 rename, focus return after relocation, and equivalent menu commands |
| List identity | A leading emoji remained ordinary title text beside a generic icon; the browser lacked an emoji font | Render one complete emoji cluster in the icon column, preserve the full stored/accessibility name, and register the complete licensed Noto COLRv1 fallback in both hosts |
| Due dates/reminders | Common choices opened full dialogs; active values had no direct removal affordance | Native anchored Today/Tomorrow/Next week/custom menus, reminder equivalents, state-aware colors, and direct remove buttons |
| Calendar | A hand-built grid exposed many day-only tab stops without the native navigation contract | Compile and use the WinUI Calendar template, including day/month/year modes, named date automation, roving focus, and keyboard navigation |
| Custom schedules | Errors appeared behind a modal in the global status surface | Validate inside the custom form, retain the draft, and focus/select the offending field |
| Steps | Step actions used a large modal, and the add glyph was not an independent submit action | Native step context menu, working add button, and focus return to the same task after promotion removes its source control |
| Text editing | Rejected blank titles could remain visually blank; notes had a small practical input area; native multiline handling consumed the save shortcut | Restore valid names, give notes the full field hit area, handle Ctrl/Command+Enter before native newline insertion, and leave pending IME composition to the native editor before submitting or dismissing fields |
| Compact layout | Long titles and settings labels lost usable width; metadata could overrun the row | Full-width headers, wrapping setting labels, single-line metadata ellipsis, and compact sort/custom-date checks |
| Typography and alignment | Secondary labels were too small or unbounded, long task names competed with row actions, and some headers lacked semantic names | Standardize 14px body editors/actions and 12px supporting labels; bound titles, wrap long actions, align icon/count columns, and name headings and inputs |
| Task entry | A recognized-date hint consumed the already narrow editor width beside details | Place the hint on its own row; preserve the editor width in both intermediate and wide three-pane layouts |
| Settings | Preference rows used checkbox presentation and long labels clipped | Compile the native WinUI ToggleSwitch template, retain pointer/keyboard/knob behavior, and wrap labels in at least 48px rows |
| Dialogs and empty states | Long content and short viewports could overlap the composer or clip help labels | Keep empty-state art inside a scroll viewport, stack compact shortcut labels, wrap modal headings and theme captions, and use the current light/dark palette |
| Theme selection | An imported or default accent outside the preset palette left every swatch unselected | Retain the exact color and expose a selected Current swatch alongside presets, including its native accessible item status |
| Mobile input area | Device-safe padding existed in the framework, but reported keyboard occlusion was not consumed and pane budgets used the unpadded width | Retain native safe-area handling, subtract horizontal insets for pane decisions, reserve the Back edge in the same padded-shell coordinates for every recognizer, and add the native resizing input-pane decorator; verify draft visibility at 390 × 360 |
| Status messages | A notification error covered the task composer and exposed an unrelated Undo action | Reserve a footer row, wrap and announce the message, and show Undo only for its relevant undoable operation |
| Removed controls | Clearing a schedule or sort could remove the focused button | Focus the retained parent action before clearing; preserve keyboard navigation after the control disappears |
| Task touch actions | Native scrolling existed without the reference mobile task actions | Add swipe-left Delete with confirmation and swipe-right My Day/Move; use a consistent list-with-arrow move glyph and keep task completion on its checkbox and existing commands |
| Hold and drag | Touch selection, sidebar organization, and long-drag lifetime were missing | Use native Holding for selection and task/list/group dragging; share mouse/touch drop planning and transfer capture to the mounted shell so virtualized-row recycling cannot misdirect a drag |
| Delayed holding | A native hold timer could outlive a captured swipe and open a menu over the revealed action | Consume stale touch/pen Holding notifications on owned gesture surfaces; only a live qualified hold can select or open a stationary sidebar menu |
| Navigation and refresh | Browser Back could leave an open pane; refresh did not have an in-app touch route | Add one guarded same-document history entry, bounded view history, shared Back dismissal, edge navigation, drawer dismissal, and pull-to-refresh that retains drafts and invokes existing explicit sync when connected |
| Attachments | The previous limit allowed multiple 25 MB files in one task | Enforce 25 MB total per task, preflight a multi-file selection, and recheck the live aggregate when adding each file |
| Offline updates | A controlling old worker could defeat preview bypass or serve incompatible runtime files | Build immutable version-addressed runtime URLs, integrity-check the precache with HTTP-cache bypass, and keep navigation responses out of runtime caches |
| Update lifecycle | An update could activate over an editing session; worker scopes/queries were too broad | Retain the normal waiting lifecycle, use exact application scope/resource matching, preserve auth queries, and complete bounded preview handoff before loading the UI |
| Cloud recovery | Document normalization can change JSON after a sync journal captures it | Canonicalize the prospective replacement before journaling, preserving local navigation preferences and validating the compare-and-replace boundary |
| Framework | The pinned submodule had a newer upstream revision | Advance the real gitlink to `062b29a537a7eef132ea9038df9d3fafb6ced6e3`; the intervening upstream changes are documentation only |

## Architecture and performance review

The six-crate division remains appropriate. Domain commands own persistent changes; the view model owns navigation, selection, transient state, services, and reactive notifications; reusable controls own the WinUI theme/templates; declarative views build the UI; hosts own persistence and platform integration. No parallel DOM task UI was added.

The list still uses one recycling data template over a stable observable source. Row projection excludes large notes and file data. Sort reversal operates on the filtered task references within each completion section and leaves saved manual positions untouched. Selection and rendering use the same ordering. Undo captures affected entities, and each sidebar topology operation remains one transaction. Group dragging reuses the existing cancellation/gesture path instead of introducing a competing input subsystem.

New implementation areas are separated into focused files: sidebar projection/drop planning, sidebar keyboard and emoji handling, scheduling commands, schedule/step menus, custom form validation, sort status, offline bootstrap/manifest generation, and focused regression scenarios. The compiled WinUI theme adds Calendar and ToggleSwitch with their dependencies rather than copying their templates into the app. Touch input remains inside FerroUI gesture recognition and native pointer capture; there is no second JavaScript touch-to-mouse bridge. Swipe motion updates a visual transform without dispatching document edits on each move. Hold-drag reuses the existing live reorder/move planners, drop markers, and bounded edge scrolling, committing at most one domain action on release.

The complete 5,038,236-byte Noto emoji font is a separate integrity-pinned browser asset downloaded in parallel with the runtime and included in the same versioned offline cache. It does not increase the WebAssembly binary. Desktop embeds the same font, while a small shared registration helper keeps networking and host paths out of the reusable controls crate. [Font provenance and license](../crates/todo-controls/assets/fonts/README.md) are recorded with the canonical asset.

CI remains a single build job and dependent Pages deployment job. Small Rust and Node suites validate state/data boundaries; one real-browser gate checks the shipped WebAssembly. There is no platform matrix, duplicate browser build, broad framework test suite, or additional deployment workflow. An exact cache key still identifies the current build inputs; a same-platform, same-framework fallback lets Cargo reuse unchanged framework artifacts when application manifests or compiled template roots change. Cargo remains responsible for dependency and compiler fingerprint validation.

## Adaptive and interaction review

The existing fit-based 760/980-pixel policy is retained. At 320/390 pixels, navigation is a temporary drawer and a selected detail/settings/suggestions surface occupies the main area. The compact detail return arrow points back to the list; the inline detail pane keeps its outward close arrow. At 760/800 pixels, list plus details remain visible and navigation overlays them. At 980/1024 pixels, the 280-pixel sidebar, usable list area, and 340-pixel details coexist. Opening the drawer never squeezes the content underneath it.

The browser gate exercises these widths, list and group pointer movement, keyboard navigation, focused editing, cancellation, menu-to-dialog transitions, native calendar selection, sorting, search return, and persistence. It also sends real browser touch events for tap, horizontal action reveal, vertical scrolling, long-press selection, manual drag across source-row recycling, two-contact cancellation, list/group organization, drawer/Back navigation, and pull-to-refresh. Light, dark, desktop, intermediate, narrow, menu, Calendar, settings, long-detail, compact-help, and touch captures form the visual review evidence. These checks establish the tested Chromium behavior; they are not certification for every physical phone, keyboard, stylus, screen reader, browser, or native operating system.

The gesture arbiter locks to a direction after touch slop. Vertical scrolling cannot later become a destructive horizontal action; swipe release reveals a button rather than immediately editing or deleting a task. A second contact is quarantined until all fingers lift so it cannot become a stray task tap. Pointer cancellation, wheel input, Escape, route change, or removal cancels pending work. Browser Back closes one active surface before traversing a valid prior list; transient renaming cancels, while ordinary note/title/settings edits flush before their surface closes. Refresh updates the local clock and uses the existing serialized account-sync path when connected; it never reloads the document.

## Remaining feature and visual differences

| Area | Current boundary | What complete parity still requires |
| --- | --- | --- |
| Typography and artwork | Bundled Inter and complete Noto COLRv1 emoji fallback, original vector icons/landscape, palette-based themes | Exact platform typography/rendering and the full reference wallpaper/theme catalog |
| Notes | Reliable plain text and multiline editing | Full web rich-text formatting, links, lists, and lossless rich HTML round trips |
| Tags/categories | Search and model/Graph fields exist | Inline clickable hashtag navigation and a complete Outlook category-editing surface |
| Recurrence | Daily/weekday/weekly/monthly/yearly/custom intervals | Arbitrary weekday sets, ending dates/counts, and every server recurrence/time-zone combination |
| Suggestions | Deterministic local suggestions | Microsoft's service-ranked personalization and complete suggestion behavior |
| Smart-list preferences | Optional All/Completed/Assigned/Flagged visibility | Every reference auto-hide/visibility setting and account-specific default |
| Bulk input/gestures | Multi-selection, documented task swipes, native hold selection, single-task and list/group touch drag, edge/drawer Back, and pull refresh | Multi-item drag, complete clipboard behavior, OS cross-app drag/drop, and physical-device/platform gesture certification |
| Microsoft accounts | Optional own-client-ID MSAL/Graph connection and explicit sync | Live account round-trip verification, automatic sync, every account/tenant policy, and complete account switching |
| Collaboration | Local list management and export; links to Microsoft's service | Invitation creation, permissions, assignment/Planner, presence, and live multi-user collaboration |
| Flagged email | Graph-projected tasks when supplied | Outlook's full mail-task integration and service-specific behavior |
| Files | Browser-local IndexedDB bytes and metadata; aggregate task budget | Microsoft attachment transfer, cross-device file sync, complete attachment backups, and native-host file support |
| Reminders | Permission-based notifications while the app runs | Closed-app/background delivery with a push/service infrastructure |
| Native distribution | Shared desktop Rust host | Tested native host parity, signed installers, store packages, and platform certification |
| Accessibility | Native control semantics, focus behavior, named actions, browser accessibility bridge | Comprehensive assistive-technology and input-method testing |

These are explicit remaining work items. The review does not label the application as a 100% Microsoft To Do clone. [Feature coverage](features.md) retains the detailed platform and service boundaries.
