use super::shell::ShellRecognizer;
use ferroui_base::input::raw::{RawPointerEventArgs, RawPointerEventType};
use todo_state::gestures::contacts::ContactEvent;

pub(super) fn process(recognizer: &ShellRecognizer, event: &RawPointerEventArgs) {
    if event.type_() == RawPointerEventType::TouchBegin
        && recognizer
            .root
            .upgrade()
            .is_some_and(|root| crate::drag::has_active(&root))
    {
        if let Some(root) = recognizer.root.upgrade() {
            crate::drag::cancel_active(&root);
        }
        recognizer
            .contacts
            .borrow_mut()
            .block_begin(event.raw_pointer_id());
        event.set_handled(true);
        return;
    }
    let kind = match event.type_() {
        RawPointerEventType::TouchBegin => ContactEvent::Begin,
        RawPointerEventType::TouchUpdate => ContactEvent::Move,
        RawPointerEventType::TouchEnd | RawPointerEventType::TouchCancel => ContactEvent::End,
        _ => return,
    };
    let active = recognizer.hub.contact()
        || recognizer.session.borrow().is_some()
        || recognizer.drag.borrow().is_some();
    let decision = recognizer
        .contacts
        .borrow_mut()
        .update(event.raw_pointer_id(), kind, active);
    if decision.cancel {
        recognizer.interrupt();
    }
    if decision.suppress {
        event.set_handled(true);
    }
}
