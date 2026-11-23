use super::shell::ShellRecognizer;
use ferroui_base::input::raw::{
    RawKeyEventArgs, RawKeyEventType, RawPointerEventArgs, RawPointerEventType,
};
use ferroui_base::input::{IInputManager, InputElement, Key};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::reactive::ObservableExt as NativeObservableExt;
use ferroui_base::{FerroLocator, LocatorExtensions, Ref};
use ferroui_controls::{Control, Grid};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
use ferroui_view::on_cleanup;
use std::{cell::RefCell, rc::Rc};
use todo_state::AppViewModel;

pub(super) fn attach(root: &Ref<Grid>, recognizer: Ref<ShellRecognizer>, vm: Rc<AppViewModel>) {
    super::environment::attach(root);
    let weak = recognizer.downgrade();
    let outside = root.add_handler_with(
        InputElement::pointer_pressed_event(),
        move |_, event| {
            if let (Some(recognizer), Some(source)) = (
                weak.upgrade(),
                event.source().and_then(|source| source.cast::<Control>()),
            ) {
                recognizer.hub.outside(&source);
            }
        },
        RoutingStrategies::TUNNEL,
        true,
    );
    let weak = recognizer.downgrade();
    let raw = FerroLocator::current()
        .get_service::<dyn IInputManager>()
        .map(|manager| {
            let tooltips = super::tooltips::HoverTooltips::default();
            manager.pre_process().subscribe_fn_guard(move |event| {
                let Some(recognizer) = weak.upgrade() else {
                    return;
                };
                let Some(root) = recognizer
                    .root
                    .upgrade()
                    .filter(|root| root.is_attached_to_visual_tree())
                else {
                    return;
                };
                if let Some(pointer) = event.downcast_ref::<RawPointerEventArgs>() {
                    if !pointer.root().root_element().is_visual_ancestor_of(&root) {
                        return;
                    }
                    let touching = recognizer.contacts.borrow().has_contacts();
                    tooltips.pointer(&root, pointer, touching);
                    if matches!(
                        pointer.type_(),
                        RawPointerEventType::LeftButtonDown
                            | RawPointerEventType::RightButtonDown
                            | RawPointerEventType::MiddleButtonDown
                    ) && touching
                        && (recognizer.interrupt() | super::pen::cancel(&recognizer, None))
                    {
                        // A press from another device interrupts contact and must
                        // not simultaneously press a button beneath the old pointer.
                        pointer.set_handled(true);
                    }
                    super::contacts::process(&recognizer, pointer);
                    if pointer.type_() == RawPointerEventType::CancelCapture {
                        recognizer.interrupt();
                        if !crate::drag::capture_handoff_active(&root) {
                            crate::drag::cancel_active(&root);
                        }
                    } else if matches!(
                        pointer.type_(),
                        RawPointerEventType::Wheel | RawPointerEventType::TouchCancel
                    ) {
                        // Touch cancellation is scoped to its device. A wheel can
                        // still scroll the destination of a conventional mouse drag.
                        recognizer.interrupt();
                    }
                } else if let Some(key) = event.downcast_ref::<RawKeyEventArgs>() {
                    if !key.root().root_element().is_visual_ancestor_of(&root) {
                        return;
                    }
                    if key.type_() == RawKeyEventType::KeyDown
                        && !recognizer.contacts.borrow().has_contacts()
                    {
                        tooltips.resume();
                    }
                    if key.type_() != RawKeyEventType::KeyDown || key.key() != Key::Escape {
                        return;
                    }
                    if crate::cancel_gestures(&root) {
                        key.set_handled(true);
                    }
                }
            })
        });
    let previous = RefCell::new(super::context::Context::capture(&vm));
    let weak_vm = Rc::downgrade(&vm);
    let weak = recognizer.downgrade();
    let changed = vm.changed().subscribe_next(move |_| {
        let (Some(recognizer), Some(vm)) = (weak.upgrade(), weak_vm.upgrade()) else {
            return;
        };
        let next = super::context::Context::capture(&vm);
        let (held_drag, invalid_drag) = {
            let drag = recognizer.drag.borrow();
            (
                drag.is_some(),
                drag.as_ref().is_some_and(|drag| !drag.valid(&vm)),
            )
        };
        let held = recognizer.hub.held() || held_drag;
        let cancel = previous.borrow().invalidates(&next, held);
        previous.replace(next);
        // Source moves/completion/sort changes can invalidate a drag without changing
        // navigation. Release the drag borrow before interruption disposes capture.
        if cancel || invalid_drag {
            recognizer.interrupt();
        }
    });
    let weak = recognizer.downgrade();
    let detached = root.detached_from_visual_tree(move |_| {
        if let Some(recognizer) = weak.upgrade() {
            recognizer.interrupt();
            *recognizer.contacts.borrow_mut() = Default::default();
        }
    });
    let weak = root.downgrade();
    on_cleanup(move || {
        changed.dispose();
        detached.dispose();
        if let Some(raw) = raw {
            raw.dispose();
        }
        recognizer.interrupt();
        if let Some(root) = weak.upgrade() {
            root.remove_handler(InputElement::pointer_pressed_event(), outside);
            root.gesture_recognizers().remove(&recognizer);
        }
    });
}
