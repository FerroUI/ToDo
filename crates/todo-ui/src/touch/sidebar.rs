use super::hub;
use crate::drag::{sidebar_identity, TouchDrag};
use ferroui_base::input::gesture_recognizers::GestureRecognizer;
use ferroui_base::input::{HoldingState, IPointer, InputElement, Pointer, PointerType};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref, StyledElementImpl,
    WeakRef,
};
use ferroui_controls::Button;
use ferroui_view::on_cleanup;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use todo_domain::{SidebarItem, View};
use todo_state::{
    gestures::{Axis, TouchTrack},
    AppViewModel,
};

#[repr(C)]
pub(super) struct SidebarRecognizer {
    pub base: GestureRecognizer,
    pub root: WeakRef<Button>,
    pub vm: Weak<AppViewModel>,
    pub session: RefCell<Option<Session>>,
}

ferro_class!(SidebarRecognizer: GestureRecognizer);
ferro_impl_classes!(SidebarRecognizer: FerroObjectImpl, StyledElementImpl);

pub(super) struct Session {
    pub item: SidebarItem,
    pub view: View,
    pub pointer: Rc<dyn IPointer>,
    pub start: Point,
    pub track: TouchTrack,
}

pub(crate) fn attach_sidebar_source(root: &Ref<Button>, vm: Rc<AppViewModel>) {
    let recognizer = instantiate(SidebarRecognizer {
        base: GestureRecognizer::construct(),
        root: root.downgrade(),
        vm: Rc::downgrade(&vm),
        session: RefCell::new(None),
    });
    root.gesture_recognizers().add(&recognizer);
    let weak = recognizer.downgrade();
    let holding = root.add_handler(InputElement::holding_event(), move |_, event| {
        if event.holding_state() == HoldingState::Started
            && matches!(event.pointer_type(), PointerType::Touch | PointerType::Pen)
        {
            // Own both live and late hold notifications. Captured input can leave the native
            // hold timer pending after scrolling/dragging; it must not open a delayed menu.
            // A valid stationary hold opens the menu only on our qualified release path.
            event.set_handled(true);
            if let Some(recognizer) = weak.upgrade() {
                recognizer.hold(event.pointer_event_args().pointer());
            }
        }
    });
    let weak = recognizer.downgrade();
    let detached = root.detached_from_visual_tree(move |_| {
        if let Some(recognizer) = weak.upgrade() {
            recognizer.cancel();
        }
    });
    let weak = root.downgrade();
    on_cleanup(move || {
        detached.dispose();
        recognizer.cancel();
        if let Some(root) = weak.upgrade() {
            root.remove_handler(InputElement::holding_event(), holding);
            root.gesture_recognizers().remove(&recognizer);
        }
    });
}

impl SidebarRecognizer {
    pub fn cancel(&self) {
        let Some(session) = self.session.borrow_mut().take() else {
            return;
        };
        if session.track.axis() == Axis::Held {
            if let Some(shell) = hub::shell(&self.base) {
                shell.feedback(None);
            }
        }
        let captured = session
            .pointer
            .as_any()
            .downcast_ref::<Pointer>()
            .and_then(Pointer::captured_gesture_recognizer)
            .is_some_and(|recognizer| std::ptr::eq(&*recognizer, &self.base));
        if captured {
            session.pointer.capture(None);
        }
    }

    fn hold(&self, pointer: &Rc<dyn IPointer>) -> bool {
        let (Some(vm), Some(root)) = (self.vm.upgrade(), self.root.upgrade()) else {
            return false;
        };
        let item = {
            let mut state = self.session.borrow_mut();
            let Some(session) = state
                .as_mut()
                .filter(|session| session.pointer.id() == pointer.id())
            else {
                return false;
            };
            if vm.view() != session.view
                || sidebar_identity(&root) != Some(session.item)
                || !TouchDrag::sidebar_available(&vm, session.item)
                || !session.track.hold()
            {
                return false;
            }
            session.item
        };
        self.capture(pointer);
        hub::claim(&self.base);
        if let Some(shell) = hub::shell(&self.base) {
            shell.feedback(Some(match item {
                SidebarItem::List(_) => "Drag to move list",
                SidebarItem::Group(_) => "Drag to move group",
            }));
        }
        true
    }
}
