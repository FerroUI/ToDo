use super::{hub, surface::Surface, task_id};
use ferroui_base::input::gesture_recognizers::GestureRecognizer;
use ferroui_base::input::{HoldingState, IPointer, InputElement, Pointer, PointerType};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref, StyledElementImpl,
};
use ferroui_controls::Grid;
use ferroui_view::on_cleanup;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use todo_domain::{Id, View};
use todo_state::{gestures::TouchTrack, AppViewModel};

#[repr(C)]
pub(super) struct TaskRecognizer {
    pub base: GestureRecognizer,
    pub vm: Weak<AppViewModel>,
    pub surface: Rc<Surface>,
    pub session: RefCell<Option<Session>>,
}

ferro_class!(TaskRecognizer: GestureRecognizer);
ferro_impl_classes!(TaskRecognizer: FerroObjectImpl, StyledElementImpl);

pub(super) struct Session {
    pub id: Id,
    pub view: View,
    pub pointer: Rc<dyn IPointer>,
    pub start: Point,
    pub track: TouchTrack,
    pub initial: f64,
    pub allow_swipe: bool,
}

impl TaskRecognizer {
    pub fn attach(root: &Ref<Grid>, vm: Rc<AppViewModel>) {
        let recognizer = instantiate(Self {
            base: GestureRecognizer::construct(),
            vm: Rc::downgrade(&vm),
            surface: Surface::new(root),
            session: RefCell::new(None),
        });
        root.gesture_recognizers().add(&recognizer);
        let weak = recognizer.downgrade();
        let holding = root.add_handler(InputElement::holding_event(), move |_, event| {
            if event.holding_state() == HoldingState::Started
                && matches!(event.pointer_type(), PointerType::Touch | PointerType::Pen)
            {
                // Captured gesture input bypasses the native routed hold timer's release
                // notification. Consume a late Started too: only our live pending session
                // may select, and a completed swipe must never fall through to a context menu.
                event.set_handled(true);
                if let Some(recognizer) = weak.upgrade() {
                    recognizer.hold(event.pointer_event_args().pointer());
                }
            }
        });
        let weak = recognizer.downgrade();
        let press = root.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, event| {
                let Some(recognizer) = weak.upgrade() else {
                    return;
                };
                if recognizer.surface.offset.get() != 0.0 && !super::task_input::is_action(event) {
                    // Tapping revealed content closes it; the original Button must not also open details.
                    if event.pointer().type_() == PointerType::Mouse {
                        recognizer.cancel();
                    }
                    event.set_handled(true);
                }
            },
            RoutingStrategies::TUNNEL,
            true,
        );
        let identity = Rc::new(std::cell::Cell::new(task_id(root)));
        let weak = recognizer.downgrade();
        let weak_root = root.downgrade();
        let changed = root.property_changed(move |_| {
            let Some(root) = weak_root.upgrade() else {
                return;
            };
            let id = task_id(&root);
            if identity.replace(id) != id {
                if let Some(recognizer) = weak.upgrade() {
                    recognizer.cancel();
                }
            }
        });
        // Virtualized templates stay mounted in the pool. Cancel their pending contact and
        // swipe offset on detach, not only when the component itself is eventually disposed.
        let weak = recognizer.downgrade();
        let detached = root.detached_from_visual_tree(move |_| {
            if let Some(recognizer) = weak.upgrade() {
                recognizer.cancel();
            }
        });
        let weak_root = root.downgrade();
        on_cleanup(move || {
            changed.dispose();
            detached.dispose();
            recognizer.cancel();
            if let Some(root) = weak_root.upgrade() {
                root.remove_handler(InputElement::holding_event(), holding);
                root.remove_handler(InputElement::pointer_pressed_event(), press);
                root.gesture_recognizers().remove(&recognizer);
            }
        });
    }

    pub fn cancel(&self) {
        let session = self.session.borrow_mut().take();
        self.surface.close();
        if let Some(session) = session {
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
    }

    pub fn active(&self) -> bool {
        self.session.borrow().is_some() || self.surface.offset.get() != 0.0
    }

    pub fn hold(&self, pointer: &Rc<dyn IPointer>) -> bool {
        let Some(vm) = self.vm.upgrade() else {
            return false;
        };
        let id = {
            let mut session = self.session.borrow_mut();
            let Some(session) = session
                .as_mut()
                .filter(|session| session.pointer.id() == pointer.id())
            else {
                return false;
            };
            if !session.track.hold() {
                return false;
            }
            session.id
        };
        self.surface.close();
        self.capture(pointer);
        hub::claim(&self.to_ref());
        if !vm.selection().contains(&id) {
            vm.toggle_task_selection(id);
        } else {
            vm.begin_selection();
        }
        true
    }
}
