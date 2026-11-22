//! Mouse hover hints must not outlive the switch to touch interaction.
use ferroui_base::data::BindingPriority;
use ferroui_base::input::raw::{RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::PointerType;
use ferroui_base::reactive::Subscription;
use ferroui_controls::{Control, ToolTip};
use std::cell::RefCell;

#[derive(Default)]
pub(super) struct HoverTooltips {
    suspended: RefCell<Option<Subscription>>,
}

impl HoverTooltips {
    pub fn pointer(&self, root: &Control, event: &RawPointerEventArgs) {
        if event.type_() == RawPointerEventType::TouchBegin {
            if self.suspended.borrow().is_some() {
                return;
            }
            // This inherited property stops the service's pending show timer. A
            // disposable override preserves the application's underlying value.
            *self.suspended.borrow_mut() = root
                .set_value_with_priority(
                    ToolTip::service_enabled_property(),
                    false,
                    BindingPriority::Animation,
                )
                .map(Subscription::new);
            // ServiceEnabled does not close an already visible popup. Collect its
            // owners before closing, since popup removal changes the visual tree.
            let owners: Vec<_> = root
                .get_visual_descendants()
                .filter_map(|visual| visual.cast::<Control>())
                .filter(|control| ToolTip::get_is_open(control))
                .collect();
            for owner in owners {
                ToolTip::set_is_open(&owner, false);
            }
        } else if matches!(
            event.type_(),
            RawPointerEventType::Move
                | RawPointerEventType::LeftButtonDown
                | RawPointerEventType::RightButtonDown
                | RawPointerEventType::MiddleButtonDown
                | RawPointerEventType::Wheel
        ) && event
            .device()
            .as_pointer_device()
            .and_then(|device| device.try_get_pointer(event))
            .is_some_and(|pointer| pointer.type_() == PointerType::Mouse)
        {
            self.resume();
        }
    }

    pub fn resume(&self) {
        let previous = self.suspended.borrow_mut().take();
        drop(previous);
    }
}
