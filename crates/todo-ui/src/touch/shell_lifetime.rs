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
use todo_state::{AppViewModel, Panel};

pub(super) fn attach(root: &Ref<Grid>, recognizer: Ref<ShellRecognizer>, vm: Rc<AppViewModel>) {
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
                    super::contacts::process(&recognizer, pointer);
                    if matches!(
                        pointer.type_(),
                        RawPointerEventType::TouchCancel
                            | RawPointerEventType::CancelCapture
                            | RawPointerEventType::Wheel
                    ) {
                        recognizer.hub.cancel();
                        recognizer.cancel();
                    }
                } else if let Some(key) = event.downcast_ref::<RawKeyEventArgs>() {
                    if key.type_() != RawKeyEventType::KeyDown
                        || key.key() != Key::Escape
                        || !key.root().root_element().is_visual_ancestor_of(&root)
                    {
                        return;
                    }
                    let task = recognizer.hub.cancel();
                    let shell = recognizer.drag.borrow().is_some()
                        || recognizer
                            .session
                            .borrow()
                            .as_ref()
                            .is_some_and(|session| session.kind.is_some());
                    recognizer.cancel();
                    if task || shell {
                        key.set_handled(true);
                    }
                }
            })
        });
    let previous = RefCell::new((vm.view(), vm.selected()));
    let weak_vm = Rc::downgrade(&vm);
    let weak = recognizer.downgrade();
    let changed = vm.changed().subscribe_next(move |_| {
        let (Some(recognizer), Some(vm)) = (weak.upgrade(), weak_vm.upgrade()) else {
            return;
        };
        let old = previous.replace((vm.view(), vm.selected()));
        let held = recognizer.hub.held() || recognizer.drag.borrow().is_some();
        if old.0 != vm.view()
            || vm.panel() != Panel::None
            || vm.rename_session().is_some()
            || (!held && (old.1 != vm.selected() || vm.selection_mode()))
        {
            recognizer.hub.cancel();
            recognizer.cancel();
        }
    });
    let weak = recognizer.downgrade();
    let detached = root.detached_from_visual_tree(move |_| {
        if let Some(recognizer) = weak.upgrade() {
            recognizer.hub.cancel();
            recognizer.cancel();
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
        recognizer.hub.cancel();
        recognizer.cancel();
        if let Some(root) = weak.upgrade() {
            root.remove_handler(InputElement::pointer_pressed_event(), outside);
            root.gesture_recognizers().remove(&recognizer);
        }
    });
}
