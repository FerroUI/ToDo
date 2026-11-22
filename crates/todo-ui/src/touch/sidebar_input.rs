use super::{
    hub,
    sidebar::{Session, SidebarRecognizer},
};
use crate::drag::{sidebar_identity, TouchDrag};
use ferroui_base::input::gesture_recognizers::GestureRecognizerImpl;
use ferroui_base::input::{
    IPointer, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerType,
};
use std::rc::Rc;
use todo_state::gestures::{Axis, TouchTrack};

impl GestureRecognizerImpl for SidebarRecognizer {
    fn pointer_pressed(this: &Self, event: &PointerPressedEventArgs) {
        if !matches!(
            event.pointer().type_(),
            PointerType::Touch | PointerType::Pen
        ) || !event.pointer().is_primary()
            || hub::shell(&this.base).is_some_and(|shell| shell.contains_back_edge(event))
        {
            return;
        }
        let (Some(vm), Some(root)) = (this.vm.upgrade(), this.root.upgrade()) else {
            return;
        };
        let Some(item) =
            sidebar_identity(&root).filter(|item| TouchDrag::sidebar_available(&vm, *item))
        else {
            return;
        };
        this.cancel();
        *this.session.borrow_mut() = Some(Session {
            item,
            view: vm.view(),
            pointer: event.pointer().clone(),
            start: event.get_position(None),
            track: TouchTrack::default(),
        });
        hub::claim(&this.base);
    }

    fn pointer_moved(this: &Self, event: &PointerEventArgs) {
        let (Some(vm), Some(root)) = (this.vm.upgrade(), this.root.upgrade()) else {
            this.cancel();
            return;
        };
        let point = event.get_position(None);
        let (axis, item, moving) = {
            let mut state = this.session.borrow_mut();
            let Some(session) = state
                .as_mut()
                .filter(|session| session.pointer.id() == event.pointer().id())
            else {
                return;
            };
            if vm.view() != session.view
                || sidebar_identity(&root) != Some(session.item)
                || !TouchDrag::sidebar_available(&vm, session.item)
            {
                drop(state);
                this.cancel();
                return;
            }
            let axis = session
                .track
                .update(point.x - session.start.x, point.y - session.start.y);
            (axis, session.item, session.track.moved_after_hold())
        };
        match axis {
            Axis::Held => {
                if moving {
                    if let (Some(shell), Some(operation)) = (
                        hub::shell(&this.base),
                        TouchDrag::sidebar(&root, &vm, item, point),
                    ) {
                        this.session.borrow_mut().take();
                        shell.begin_sidebar_drag(event.pointer().clone(), operation, &root);
                    }
                }
                event.set_handled(true);
            }
            Axis::Pending => {}
            _ => this.cancel(), // Scroll or drawer dismissal won before Holding.
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
        let session = this.session.borrow_mut().take().unwrap();
        if session.track.axis() == Axis::Held {
            if let Some(shell) = hub::shell(&this.base) {
                shell.feedback(None);
            }
            event.set_handled(true);
            if !session.track.moved_after_hold() {
                if let Some(root) = this.root.upgrade() {
                    crate::menus::open(&root);
                }
            }
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
