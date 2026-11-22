# Gesture control integration at the current FerroUI revision

## Decision

Keep the app-owned touch recognizers and action presentation while FerroUI is pinned to
[`062b29a537a7eef132ea9038df9d3fafb6ced6e3`](https://github.com/wieslawsoltes/FerroUI/tree/062b29a537a7eef132ea9038df9d3fafb6ced6e3).
The recognizers coordinate row actions, native scrolling, holding, manual drag, edge navigation,
and cancellation through one mounted shell. A direct replacement with `SwipeControl` and
`RefreshContainer` would change concrete input and cancellation behavior described below.

This is a source-level integration assessment for that revision. It does not establish visual
or behavioral parity. Production code was not changed by this assessment.

## Native controls available

FerroUI provides declarative builders for both controls and WinUI-derived templates:

- [`SwipeControl.xaml`](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Themes.WinUI/Controls/SwipeControl.xaml)
  supplies the content/command layers and a `SwipeItemStyle` for templated `CommandBarButton`
  items. Reveal-mode `RightItems` map to Delete; reveal-mode `LeftItems` map to My Day and Move.
  An invocation can use the live control identity through
  [`SwipeItemInvokedEventArgs`](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Controls/swipe_control/swipe_items.rs).
- [`RefreshContainer.xaml`](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Themes.WinUI/Controls/RefreshContainer.xaml)
  hosts scrollable content and the native
  [`RefreshVisualizer`](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Themes.WinUI/Controls/RefreshVisualizer.xaml).
  Its `RefreshRequested` event supports a completion deferral, suitable for keeping feedback
  active until the app's asynchronous refresh finishes.

Browser input reaches these same Rust controls. The browser host forwards DOM pointer events,
including cancellation, through the native input bridge
([TypeScript routing](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/Browser/FerroUI.Browser/webapp/modules/ferroui/input.ts#L660-L719),
[Rust cancellation](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/Browser/FerroUI.Browser/browser_input_handler.rs#L429-L464)).
There is no separate browser-control implementation to enable.

## SwipeControl integration gaps

1. **The task title handles the press before SwipeControl receives it.** Native
   [`Button::on_pointer_pressed`](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Controls/button.rs#L302-L320)
   marks the press handled. `SwipeControl` starts its interaction only from its ordinary
   [`on_pointer_pressed`](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Controls/swipe_control/swipe_control.rs#L214-L228)
   override, reached by a class handler that
   [excludes handled events](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Base/interactivity/routed_event.rs#L350-L377).
   The app's title is a Button, so a declarative wrapper alone would leave that main swipe
   surface inactive.
2. **Redirection captures before directional intent is known.**
   [`try_redirect_for_manipulation_point`](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Base/rendering/composition/interactions/visual_interaction_source.rs#L443-L506)
   immediately captures to the interaction source. Adding an unconditional handled-events-too
   bridge would therefore transfer the title's capture on press, before a normal tap, holding,
   scrolling, or horizontal swipe is distinguished. The existing app makes that decision before
   capture and reserves the shell's Back edge.
3. **Cancellation can settle open.** Capture loss invokes
   [`manipulation_ended(None)`](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Base/rendering/composition/interactions/visual_interaction_source.rs#L484-L505),
   which ends the interaction with zero velocity. The tracker still enters inertia and applies
   the control's [open resting-value modifiers](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Controls/swipe_control/swipe_control.rs#L478-L533).
   Consequently a cancelled reveal past the threshold can remain open. Public
   [`close()`](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Controls/swipe_control/swipe_control.rs#L308-L347)
   explicitly returns during interaction; the tracker and immediate reset are private.
   Releasing capture and then closing is a possible coordinated adapter operation, but the
   control exposes no atomic cancel/reset contract.

The native templates, commands, and animations are useful migration targets. Safe integration
also needs deferred gesture acquisition through handled Button content, explicit cancellation,
and continued app ownership of multi-contact quarantine, holding, and virtualized drag lifetime.

## RefreshContainer cancellation and direction

The default
[`ScrollablePullGestureRecognizer`](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Controls/pull_to_refresh/scrollable_pull_gesture_recognizer.rs#L55-L117)
starts on any positive delta along its pull direction at the scroll edge. It has no directional
slop or orthogonal-axis rejection. Its capture-lost path calls the same `end_pull()` used by a
normal release, and another press replaces the tracked pointer.

The provider's
[`interacting_state_exited`](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Controls/pull_to_refresh/refresh_info_provider.rs#L227-L232)
sets interacting to false before resetting the ratio. If the visualizer is Pending, that
notification immediately
[`request_refresh()`](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Controls/pull_to_refresh/refresh_visualizer.rs#L597-L610).
Thus cancellation after crossing the threshold can initiate a refresh. The app requires
cancellation to clear the gesture without invoking the operation.

## Can RefreshVisualizer be reused just for presentation?

**The current public provider API does not support that composition directly.**
`RefreshInfoProvider::new` and `set_interaction_ratio` are public, but the visualizer's provider
[property, getter, and setter are crate-private](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Controls/pull_to_refresh/refresh_visualizer.rs#L183-L194).
The public state property is read-only. `RefreshContainer` also keeps its
[adapter attachment API crate-private](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Controls/pull_to_refresh/refresh_container.rs#L228-L235).

Calling `RefreshVisualizer::request_refresh()` without a provider raises the event and changes
state, but does not produce its normal animated feedback:
[`update_content()`](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Controls/pull_to_refresh/refresh_visualizer.rs#L331-L345)
returns unless the provider supplies a composition visual. A standalone instance cannot replace
the current pull indicator through the supported API alone.

With a public provider-attachment seam, the existing recognizer could drive the native
visualizer safely: on cancellation, set the ratio to zero **before** ending interaction; on a
qualified release, end interaction while the ratio exceeds the execution threshold; complete
the refresh deferral when host work finishes. This follows the native
[state transitions](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Controls/pull_to_refresh/refresh_visualizer.rs#L549-L610).

An app adapter could instead wrap `RefreshContainer`, remove its installed pull recognizer,
and supply pull events to the default adapter's target. That requires coordinating the
[adapter's template/loaded lifecycle](https://github.com/wieslawsoltes/FerroUI/blob/062b29a537a7eef132ea9038df9d3fafb6ced6e3/src/FerroUI.Controls/pull_to_refresh/scroll_viewer_i_refresh_info_provider_adapter.rs#L105-L199)
and preserving zero-before-end cancellation. It is a more coupled integration than a public
provider seam and is not used by the app.

Reassess this boundary when the pinned framework exposes the required cancellation and
custom-provider hooks. Validate any migration with real pointer input through handled task
Buttons, ordinary vertical scrolling, holding/manual drag, multi-contact cancellation,
virtualized source recycling, and cancellation after the refresh threshold.
