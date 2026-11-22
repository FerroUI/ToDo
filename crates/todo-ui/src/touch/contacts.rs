use super::shell::ShellRecognizer;
use ferroui_base::input::raw::{RawPointerEventArgs, RawPointerEventType};
use todo_state::gestures::contacts::{ContactEvent, Contacts};

pub(super) fn process(recognizer: &ShellRecognizer, event: &RawPointerEventArgs) {
    let kind = match event.type_() {
        RawPointerEventType::TouchBegin => ContactEvent::Begin,
        RawPointerEventType::TouchUpdate => ContactEvent::Move,
        RawPointerEventType::TouchEnd | RawPointerEventType::TouchCancel => ContactEvent::End,
        RawPointerEventType::CancelCapture => {
            *recognizer.contacts.borrow_mut() = Contacts::default();
            return;
        }
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
        let pointer = recognizer
            .hub
            .pointer()
            .or_else(|| {
                recognizer
                    .session
                    .borrow()
                    .as_ref()
                    .map(|session| session.pointer.clone())
            })
            .or_else(|| {
                recognizer
                    .drag
                    .borrow()
                    .as_ref()
                    .map(|drag| drag.pointer.clone())
            });
        recognizer.hub.cancel();
        recognizer.cancel();
        // A pending contact can still belong to a native Button. Clear its implicit capture
        // too, so its later End disposes the pointer without clicking the old target.
        if let Some(pointer) = pointer {
            pointer.capture(None);
        }
    }
    if decision.suppress {
        event.set_handled(true);
    }
}
