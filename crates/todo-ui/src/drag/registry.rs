use super::native::NativeDrag;
use ferroui_controls::Control;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

thread_local! {
    static ACTIVE: RefCell<Vec<Weak<NativeDrag>>> = const { RefCell::new(Vec::new()) };
}

pub(super) fn register(session: &Rc<NativeDrag>) {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        active.retain(|session| session.upgrade().is_some_and(|session| session.active()));
        active.push(Rc::downgrade(session));
    });
}

fn sessions(root: &Control) -> Vec<Rc<NativeDrag>> {
    ACTIVE.with(|active| {
        active
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .filter(|session| session.active() && session.matches_root(root))
            .collect()
    })
}

pub(crate) fn has_active(root: &Control) -> bool {
    !sessions(root).is_empty()
}

pub(crate) fn cancel_active(root: &Control) -> bool {
    // Do not retain the registry borrow while capture loss or host history can re-enter.
    let sessions = sessions(root);
    let canceled = !sessions.is_empty();
    for session in sessions {
        session.cancel();
    }
    canceled
}
