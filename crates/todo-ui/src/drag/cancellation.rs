use ferroui_base::input::raw::{
    RawKeyEventArgs, RawKeyEventType, RawPointerEventArgs, RawPointerEventType,
};
use ferroui_base::input::{IInputManager, Key, RawInputModifiers};
use ferroui_base::reactive::{ObservableExt, Subscription};
use ferroui_base::{FerroLocator, LocatorExtensions};
use std::cell::Cell;
use std::rc::Rc;

/// Invalidate before the platform drag observer can raise another Drop while its completion
/// continuation is queued. The guard owns no controls and unsubscribes even on task teardown.
pub(super) struct Cancellation {
    _subscription: Option<Subscription>,
    active: Rc<Cell<bool>>,
}

impl Cancellation {
    pub(super) fn watch(active: Rc<Cell<bool>>) -> Self {
        let flag = active.clone();
        let subscription = FerroLocator::current()
            .get_service::<dyn IInputManager>()
            .map(|manager| {
                manager.pre_process().subscribe_fn_guard(move |event| {
                    let escape = event.downcast_ref::<RawKeyEventArgs>().is_some_and(|key| {
                        key.type_() == RawKeyEventType::KeyDown && key.key() == Key::Escape
                    });
                    let canceled =
                        event
                            .downcast_ref::<RawPointerEventArgs>()
                            .is_some_and(|pointer| match pointer.type_() {
                                RawPointerEventType::LeftButtonDown
                                | RawPointerEventType::RightButtonDown
                                | RawPointerEventType::MiddleButtonDown
                                | RawPointerEventType::NonClientLeftButtonDown
                                | RawPointerEventType::RightButtonUp
                                | RawPointerEventType::MiddleButtonUp => true,
                                RawPointerEventType::Move => {
                                    let buttons = RawInputModifiers::LEFT_MOUSE_BUTTON
                                        | RawInputModifiers::RIGHT_MOUSE_BUTTON
                                        | RawInputModifiers::MIDDLE_MOUSE_BUTTON;
                                    pointer.input_modifiers() & buttons
                                        != RawInputModifiers::LEFT_MOUSE_BUTTON
                                }
                                _ => false,
                            });
                    if escape || canceled {
                        flag.set(false);
                    }
                })
            });
        Self {
            _subscription: subscription,
            active,
        }
    }
}

impl Drop for Cancellation {
    fn drop(&mut self) {
        self.active.set(false);
    }
}
