use super::{
    hub,
    task::{Session, TaskRecognizer},
    task_id,
};
use crate::drag::TouchDrag;
use ferroui_base::input::gesture_recognizers::GestureRecognizerImpl;
use ferroui_base::input::{
    IPointer, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::{Button, CheckBox, Control, TextBox};
use std::rc::Rc;
use todo_state::{
    Panel,
    gestures::{Axis, EDGE_WIDTH, TouchTrack, settle_swipe, swipe_offset},
};

impl GestureRecognizerImpl for TaskRecognizer {
    fn pointer_pressed(this: &Self, event: &PointerPressedEventArgs) {
        if !matches!(
            event.pointer().type_(),
            PointerType::Touch | PointerType::Pen
        ) || !event.pointer().is_primary()
        {
            return;
        }
        let Some(root) = this.surface.root.upgrade() else {
            return;
        };
        let Some(vm) = this.vm.upgrade() else { return };
        let Some(id) = task_id(&root) else { return };
        let point = event.get_position(None);
        if point.x <= EDGE_WIDTH
            || is_action(event)
            || is_editor_or_toggle(event)
            || vm.panel() != Panel::None
            || vm.rename_session().is_some()
            || vm.state().task(id).is_none()
        {
            return;
        }
        if this.session.borrow().is_some() {
            this.cancel();
        }
        *this.session.borrow_mut() = Some(Session {
            id,
            view: vm.view(),
            pointer: event.pointer().clone(),
            start: point,
            track: TouchTrack::default(),
            initial: this.surface.offset.get(),
            allow_swipe: !vm.selection_mode(),
        });
        hub::claim(&this.to_ref());
    }

    fn pointer_moved(this: &Self, event: &PointerEventArgs) {
        let Some(vm) = this.vm.upgrade() else {
            this.cancel();
            return;
        };
        let point = event.get_position(None);
        let (axis, offset, held_id) = {
            let mut sessions = this.session.borrow_mut();
            let Some(session) = sessions
                .as_mut()
                .filter(|session| session.pointer.id() == event.pointer().id())
            else {
                return;
            };
            if vm.view() != session.view
                || vm.panel() != Panel::None
                || vm.state().task(session.id).is_none()
            {
                drop(sessions);
                this.cancel();
                return;
            }
            let axis = session
                .track
                .update(point.x - session.start.x, point.y - session.start.y);
            (
                axis,
                if session.allow_swipe {
                    Some(swipe_offset(session.initial, session.track.x))
                } else {
                    None
                },
                session.track.moved_after_hold().then_some(session.id),
            )
        };
        match axis {
            Axis::Horizontal if offset.is_some() => {
                this.capture(event.pointer());
                this.surface.render(offset.unwrap());
                event.set_handled(true);
            }
            Axis::Held => {
                if let (Some(id), Some(root), Some(shell)) =
                    (held_id, this.surface.root.upgrade(), hub::shell(this))
                {
                    if let Some(operation) = TouchDrag::new(&root, &vm, id, point) {
                        // Capture-lost is synchronous. Remove the row's session before handing
                        // capture to the shell, so a recycled row cannot cancel an active drag.
                        this.session.borrow_mut().take();
                        shell.begin_drag(event.pointer().clone(), operation, &this.surface);
                    }
                }
                event.set_handled(true);
            }
            Axis::Vertical | Axis::Canceled | Axis::Horizontal => this.cancel(),
            Axis::Pending => {}
        }
    }

    fn pointer_released(this: &Self, event: &PointerReleasedEventArgs) {
        if !this
            .session
            .borrow()
            .as_ref()
            .is_some_and(|session| session.pointer.id() == event.pointer().id())
        {
            return;
        }
        let mut session = this.session.borrow_mut().take().unwrap();
        let axis = session.track.axis();
        if axis == Axis::Horizontal && session.allow_swipe {
            let point = event.get_position(None);
            session
                .track
                .update(point.x - session.start.x, point.y - session.start.y);
            this.surface
                .render(settle_swipe(swipe_offset(session.initial, session.track.x)));
        } else if session.initial != 0.0 || axis == Axis::Held {
            this.surface.close();
        }
        if axis != Axis::Pending || session.initial != 0.0 {
            event.set_handled(true);
        }
    }

    fn pointer_capture_lost(this: &Self, pointer: &Rc<dyn IPointer>) {
        if this
            .session
            .borrow()
            .as_ref()
            .is_some_and(|session| session.pointer.id() == pointer.id())
        {
            this.cancel();
        }
    }
}

pub(super) fn is_action(event: &PointerEventArgs) -> bool {
    event
        .source()
        .and_then(|source| source.cast::<Control>())
        .is_some_and(|source| {
            source
                .get_self_and_visual_ancestors()
                .filter_map(|visual| visual.cast::<Button>())
                .any(|button| button.name().is_some_and(|name| name.starts_with("Swipe")))
        })
}

fn is_editor_or_toggle(event: &PointerEventArgs) -> bool {
    event
        .source()
        .and_then(|source| source.cast::<Control>())
        .is_some_and(|source| {
            source.get_self_and_visual_ancestors().any(|visual| {
                visual.is::<TextBox>() || visual.is::<CheckBox>() || visual.is::<ToggleButton>()
            })
        })
}
