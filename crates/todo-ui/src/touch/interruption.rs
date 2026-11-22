//! Explicit interruption differs from yielding an undecided axis to native scrolling.
use super::shell::ShellRecognizer;
use ferroui_base::Ref;
use ferroui_controls::Control;

pub(crate) fn has_active(root: &Control) -> bool {
    recognizer(root).is_some_and(|recognizer| recognizer.active())
}

pub(crate) fn cancel_active(root: &Control) -> bool {
    recognizer(root).is_some_and(|recognizer| recognizer.interrupt())
}

pub(crate) fn interrupted_contacts(root: &Control) -> Vec<i64> {
    recognizer(root)
        .map(|recognizer| recognizer.contacts.borrow().interrupted_ids())
        .unwrap_or_default()
}

fn recognizer(root: &Control) -> Option<Ref<ShellRecognizer>> {
    let root = root.find_descendant_of_type_where::<Control>(true, |control| {
        control.name().as_deref() == Some("TodoApp")
    })?;
    root.gesture_recognizers()
        .to_vec()
        .into_iter()
        .find_map(|recognizer| recognizer.cast::<ShellRecognizer>())
}

impl ShellRecognizer {
    pub fn active(&self) -> bool {
        self.hub.has_active() || self.session.borrow().is_some() || self.drag.borrow().is_some()
    }

    pub fn interrupt(&self) -> bool {
        if !self.active() {
            return false;
        }
        // A pending native Button still owns implicit capture. Collect pointers before
        // cancelling recognizers, and release all borrows before capture-loss callbacks.
        let pointers = [
            self.hub.pointer(),
            self.session
                .borrow()
                .as_ref()
                .map(|session| session.pointer.clone()),
            self.drag.borrow().as_ref().map(|drag| drag.pointer.clone()),
        ];
        self.contacts.borrow_mut().quarantine();
        self.hub.cancel();
        self.cancel();
        for pointer in pointers.into_iter().flatten() {
            pointer.capture(None);
        }
        self.sync_back();
        true
    }

    pub fn sync_back(&self) {
        if let (Some(vm), Some(root)) = (self.vm.upgrade(), self.root.upgrade()) {
            crate::shell::navigation::sync_host(&vm, Some(&root));
        }
    }
}
