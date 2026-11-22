use super::native::NativeDrag;
use ferroui_base::input::raw::{
    RawKeyEventArgs, RawKeyEventType, RawPointerEventArgs, RawPointerEventType,
};
use ferroui_base::input::{IInputManager, Key, RawInputModifiers};
use ferroui_base::reactive::{ObservableExt as NativeObservableExt, Subscription};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
use std::rc::Rc;
use todo_state::AppViewModel;

/// Invalidate before the platform observer can raise a Drop. Model changes are observed
/// immediately so returning to a prior route/parent never rearms an interrupted gesture.
pub(super) struct Cancellation {
    _input: Option<Subscription>,
    _model: ferroui_system::Subscription,
    _detached: Option<Subscription>,
}

impl Cancellation {
    pub fn watch(session: &Rc<NativeDrag>, vm: &Rc<AppViewModel>) -> Self {
        let weak = Rc::downgrade(session);
        let input = FerroLocator::current()
            .get_service::<dyn IInputManager>()
            .map(|manager| {
                manager.pre_process().subscribe_fn_guard(move |event| {
                    let Some(session) = weak.upgrade().filter(|session| session.active()) else {
                        return;
                    };
                    let Some(root) = session.root.upgrade() else {
                        session.cancel();
                        return;
                    };
                    if !event.root().root_element().is_visual_ancestor_of(&root) {
                        return;
                    }
                    if let Some(key) = event.downcast_ref::<RawKeyEventArgs>() {
                        if key.type_() == RawKeyEventType::KeyDown && key.key() == Key::Escape {
                            session.cancel();
                            key.set_handled(true);
                        }
                    } else if let Some(pointer) = event.downcast_ref::<RawPointerEventArgs>() {
                        let handoff = pointer.type_() == RawPointerEventType::CancelCapture
                            && session.capture_handoff_active();
                        if !handoff && cancels(pointer) {
                            session.cancel();
                        }
                    }
                })
            });
        let weak = Rc::downgrade(session);
        let owner = Rc::downgrade(vm);
        let model = vm.changed().subscribe_next_guard(move |_| {
            if let (Some(session), Some(vm)) = (weak.upgrade(), owner.upgrade()) {
                session.valid(&vm);
            }
        });
        let weak = Rc::downgrade(session);
        let detached = session.root.upgrade().map(|root| {
            root.detached_from_visual_tree(move |_| {
                if let Some(session) = weak.upgrade() {
                    session.cancel();
                }
            })
        });
        Self {
            _input: input,
            _model: model,
            _detached: detached,
        }
    }
}

fn cancels(pointer: &RawPointerEventArgs) -> bool {
    match pointer.type_() {
        RawPointerEventType::LeftButtonDown
        | RawPointerEventType::RightButtonDown
        | RawPointerEventType::MiddleButtonDown
        | RawPointerEventType::NonClientLeftButtonDown
        | RawPointerEventType::RightButtonUp
        | RawPointerEventType::MiddleButtonUp
        | RawPointerEventType::CancelCapture => true,
        RawPointerEventType::Move => {
            let buttons = RawInputModifiers::LEFT_MOUSE_BUTTON
                | RawInputModifiers::RIGHT_MOUSE_BUTTON
                | RawInputModifiers::MIDDLE_MOUSE_BUTTON;
            pointer.input_modifiers() & buttons != RawInputModifiers::LEFT_MOUSE_BUTTON
        }
        _ => false,
    }
}
