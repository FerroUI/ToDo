//! Retire a cancelled platform pen through its native device, including a
//! captured Button/editor that is not owned by an app gesture recognizer.
use super::shell::ShellRecognizer;
use ferroui_base::input::raw::{RawPointerEventArgs, RawPointerEventType};
use ferroui_base::input::{IInputDevice, IInputManager, IInputRoot, RawInputModifiers};
use ferroui_base::{FerroLocator, LocatorExtensions, Point};
use std::{
    collections::BTreeMap,
    rc::{Rc, Weak},
};

pub(super) struct PenContact {
    device: Weak<dyn IInputDevice>,
    root: Weak<dyn IInputRoot>,
    timestamp: u64,
    position: Point,
}

pub(super) type PenContacts = BTreeMap<i64, PenContact>;

pub(super) fn remember(recognizer: &ShellRecognizer, event: &RawPointerEventArgs) {
    recognizer.pens.borrow_mut().insert(
        event.raw_pointer_id(),
        PenContact {
            device: Rc::downgrade(event.device()),
            root: Rc::downgrade(event.root()),
            timestamp: event.timestamp(),
            position: event.position(),
        },
    );
}

pub(super) fn cancel(recognizer: &ShellRecognizer, id: Option<i64>) -> bool {
    // No borrow survives dispatch: capture loss can reconcile the shell.
    let contacts = if let Some(id) = id {
        recognizer
            .pens
            .borrow_mut()
            .remove(&id)
            .map(|contact| vec![(id, contact)])
            .unwrap_or_default()
    } else {
        std::mem::take(&mut *recognizer.pens.borrow_mut())
            .into_iter()
            .collect()
    };
    let handled = !contacts.is_empty();
    if let Some(manager) = FerroLocator::current().get_service::<dyn IInputManager>() {
        for (id, contact) in contacts {
            let (Some(device), Some(root)) = (contact.device.upgrade(), contact.root.upgrade())
            else {
                continue;
            };
            // PenDevice disposes this raw ID on LeaveWindow. It emits capture
            // loss, never a release/click, and ignores a later unmatched Up.
            let event = Rc::new(RawPointerEventArgs::new(
                device,
                contact.timestamp,
                root,
                RawPointerEventType::LeaveWindow,
                contact.position,
                RawInputModifiers::NONE,
            ));
            event.set_raw_pointer_id(id);
            manager.process_input(event);
        }
    }
    handled
}

#[cfg(test)]
#[path = "pen_tests.rs"]
mod tests;
