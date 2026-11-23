use super::shell::ShellRecognizer;
use ferroui_base::input::raw::{RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::PenDevice;
use todo_state::gestures::contacts::ContactEvent;

pub(super) fn process(recognizer: &ShellRecognizer, event: &RawPointerEventArgs) {
    let pen = event.device().as_any().is::<PenDevice>();
    let kind = match event.type_() {
        RawPointerEventType::TouchBegin => ContactEvent::Begin,
        RawPointerEventType::TouchUpdate => ContactEvent::Move,
        RawPointerEventType::TouchEnd | RawPointerEventType::TouchCancel => ContactEvent::End,
        RawPointerEventType::LeftButtonDown
        | RawPointerEventType::RightButtonDown
        | RawPointerEventType::MiddleButtonDown
        | RawPointerEventType::XButton1Down
        | RawPointerEventType::XButton2Down
            if pen =>
        {
            ContactEvent::Begin
        }
        RawPointerEventType::LeftButtonUp
        | RawPointerEventType::RightButtonUp
        | RawPointerEventType::MiddleButtonUp
        | RawPointerEventType::XButton1Up
        | RawPointerEventType::XButton2Up
        | RawPointerEventType::LeaveWindow
        | RawPointerEventType::CancelCapture
            if pen =>
        {
            ContactEvent::End
        }
        RawPointerEventType::Move
            if pen && recognizer.contacts.borrow().has_pen(event.raw_pointer_id()) =>
        {
            ContactEvent::Move
        }
        _ => return,
    };
    if matches!(kind, ContactEvent::Begin)
        && recognizer
            .root
            .upgrade()
            .is_some_and(|root| crate::drag::has_active(&root))
    {
        if let Some(root) = recognizer.root.upgrade() {
            crate::drag::cancel_active(&root);
        }
        if pen {
            recognizer
                .contacts
                .borrow_mut()
                .block_pen_begin(event.raw_pointer_id());
        } else {
            recognizer
                .contacts
                .borrow_mut()
                .block_begin(event.raw_pointer_id());
        }
        event.set_handled(true);
        return;
    }
    let active = recognizer.hub.contact()
        || recognizer.session.borrow().is_some()
        || recognizer.drag.borrow().is_some();
    let decision = if pen {
        if matches!(kind, ContactEvent::Begin) && event.handled() {
            recognizer
                .contacts
                .borrow_mut()
                .block_pen_begin(event.raw_pointer_id())
        } else {
            recognizer
                .contacts
                .borrow_mut()
                .update_pen(event.raw_pointer_id(), kind, active)
        }
    } else {
        recognizer
            .contacts
            .borrow_mut()
            .update(event.raw_pointer_id(), kind, active)
    };
    if pen {
        if matches!(kind, ContactEvent::End) {
            recognizer.pens.borrow_mut().remove(&event.raw_pointer_id());
        } else if !decision.suppress {
            super::pen::remember(recognizer, event);
        }
    }
    if decision.cancel {
        recognizer.interrupt();
        super::pen::cancel(recognizer, None);
    }
    if decision.suppress {
        event.set_handled(true);
    }
}
