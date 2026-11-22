use super::shell::{Kind, Session, ShellRecognizer};
use ferroui_base::input::gesture_recognizers::GestureRecognizerImpl;
use ferroui_base::input::{
    IPointer, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use ferroui_controls::primitives::ScrollBar;
use ferroui_controls::{Control, Grid, ListBox, ScrollViewer, TextBox};
use std::rc::Rc;
use todo_state::{
    gestures::{Axis, TouchTrack},
    Panel, NAVIGATION_WIDTH,
};

impl GestureRecognizerImpl for ShellRecognizer {
    fn pointer_pressed(this: &Self, event: &PointerPressedEventArgs) {
        if !matches!(
            event.pointer().type_(),
            PointerType::Touch | PointerType::Pen
        ) || !event.pointer().is_primary()
            || is_native_drag_source(event)
        {
            return;
        }
        let Some(vm) = this.vm.upgrade() else { return };
        let Some(root) = this.root.upgrade() else {
            return;
        };
        let point = event.get_position(Some(&root));
        if point.y < 36.0 {
            return;
        }
        let back = this.contains_back_edge(event) && (this.can_back)();
        let drawer = vm.navigation_overlay_open() && point.x <= NAVIGATION_WIDTH;
        let refresh = vm.panel() == Panel::None
            && !vm.navigation_overlay_open()
            && !vm.refreshing()
            && at_list_top(event, &root);
        if back || drawer || refresh {
            *this.session.borrow_mut() = Some(Session {
                pointer: event.pointer().clone(),
                start: event.get_position(None),
                track: TouchTrack::default(),
                back,
                drawer,
                refresh,
                kind: None,
            });
            this.sync_back();
        }
    }

    fn pointer_moved(this: &Self, event: &PointerEventArgs) {
        let point = event.get_position(None);
        let drag_update = {
            let mut drag = this.drag.borrow_mut();
            drag.as_mut()
                .filter(|drag| drag.pointer.id() == event.pointer().id())
                .map(|drag| drag.update(point))
        };
        if let Some(message) = drag_update {
            if let Some(message) = message {
                this.feedback(Some(message));
            }
            event.set_handled(true);
            return;
        }
        let (kind, ready, abandon, rejected_horizontal) = {
            let mut state = this.session.borrow_mut();
            let Some(session) = state
                .as_mut()
                .filter(|session| session.pointer.id() == event.pointer().id())
            else {
                return;
            };
            let axis = session
                .track
                .update(point.x - session.start.x, point.y - session.start.y);
            if session.kind.is_none() {
                session.kind = match axis {
                    Axis::Horizontal if session.back && session.track.x > 0.0 => Some(Kind::Back),
                    Axis::Horizontal if session.drawer && session.track.x < 0.0 => {
                        Some(Kind::Drawer)
                    }
                    Axis::Vertical if session.refresh && session.track.y > 0.0 => {
                        Some(Kind::Refresh)
                    }
                    _ => None,
                };
            }
            (
                session.kind,
                session.kind.is_some_and(|kind| kind.ready(&session.track)),
                axis != Axis::Pending && session.kind.is_none(),
                matches!(axis, Axis::Horizontal | Axis::Canceled),
            )
        };
        if abandon {
            if rejected_horizontal {
                // Child gestures have already had their turn on this routed move.
                // A rejected horizontal drawer swipe must not retain Button capture
                // and turn into a navigation click when released on the same row.
                this.interrupt();
                event.set_handled(true);
            } else {
                this.cancel(); // Native vertical scrolling keeps its tracking/capture.
            }
            return;
        }
        let Some(kind) = kind else { return };
        this.capture(event.pointer());
        event.set_handled(true);
        this.feedback(Some(match (kind, ready) {
            (Kind::Refresh, false) => "↓ Pull to refresh",
            (Kind::Refresh, true) => "↻ Release to refresh",
            (Kind::Drawer, _) => "← Close navigation",
            (Kind::Back, _) => "← Back",
        }));
    }

    fn pointer_released(this: &Self, event: &PointerReleasedEventArgs) {
        if this
            .drag
            .borrow()
            .as_ref()
            .is_some_and(|drag| drag.pointer.id() == event.pointer().id())
        {
            let drag = this.drag.borrow_mut().take().unwrap();
            this.feedback(None);
            this.sync_back();
            event.set_handled(true);
            drag.finish(event.get_position(None));
            return;
        }
        if !this
            .session
            .borrow()
            .as_ref()
            .is_some_and(|session| session.pointer.id() == event.pointer().id())
        {
            return;
        }
        let mut session = this.session.borrow_mut().take().unwrap();
        this.feedback(None);
        this.sync_back();
        let Some(kind) = session.kind else { return };
        let point = event.get_position(None);
        session
            .track
            .update(point.x - session.start.x, point.y - session.start.y);
        event.set_handled(true);
        if !kind.ready(&session.track) {
            return;
        }
        match kind {
            Kind::Refresh => {
                if let Some(vm) = this.vm.upgrade() {
                    vm.refresh();
                }
            }
            Kind::Back if (this.can_back)() => {
                (this.on_back)();
            }
            Kind::Drawer => {
                if this
                    .vm
                    .upgrade()
                    .is_some_and(|vm| vm.navigation_overlay_open())
                {
                    (this.on_back)();
                }
            }
            _ => {}
        }
    }

    fn pointer_capture_lost(this: &Self, pointer: &Rc<dyn IPointer>) {
        if this
            .drag
            .borrow()
            .as_ref()
            .is_some_and(|drag| drag.pointer.id() == pointer.id())
            || this
                .session
                .borrow()
                .as_ref()
                .is_some_and(|session| session.pointer.id() == pointer.id())
        {
            this.cancel();
        }
    }
}

fn is_native_drag_source(event: &PointerEventArgs) -> bool {
    // TextBox claims touch selection only after its own movement threshold. The shell's
    // smaller threshold must not steal that capture; scrolling chrome also owns its drags.
    event
        .source()
        .and_then(|source| source.cast::<Control>())
        .is_some_and(|source| {
            source
                .get_self_and_visual_ancestors()
                .any(|visual| visual.is::<TextBox>() || visual.is::<ScrollBar>())
        })
}

fn at_list_top(event: &PointerEventArgs, root: &Grid) -> bool {
    let Some(list) = root.find_descendant_of_type_where::<ListBox>(false, |list| {
        list.name().as_deref() == Some("TaskList") && list.is_effectively_visible()
    }) else {
        return false;
    };
    let point = event.get_position(Some(&list));
    if point.x < 0.0
        || point.y < 0.0
        || point.x >= list.bounds().width
        || point.y >= list.bounds().height
    {
        return false;
    }
    // Empty-state artwork is a sibling overlay, so use the actual viewport bounds as well as
    // the pointer source. This keeps refresh available when the task list is empty.
    list.find_descendant_of_type::<ScrollViewer>(false)
        .is_some_and(|scroll| scroll.offset().y <= 0.5)
}
