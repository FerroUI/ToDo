use super::{shell::ShellRecognizer, task::TaskRecognizer};
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::Control;
use std::cell::RefCell;
use todo_state::gestures::Axis;

/// One coordinator belongs to one mounted shell. It never retains a row or application strongly.
#[derive(Default)]
pub(super) struct Hub {
    active: RefCell<Option<WeakRef<TaskRecognizer>>>,
}

impl Hub {
    pub fn active(&self) -> Option<Ref<TaskRecognizer>> {
        self.active.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub fn cancel(&self) -> bool {
        let active = self
            .active
            .borrow_mut()
            .take()
            .and_then(|weak| weak.upgrade());
        let handled = active.as_ref().is_some_and(|active| active.active());
        if let Some(active) = active {
            active.cancel();
        }
        handled
    }

    pub fn held(&self) -> bool {
        self.active().is_some_and(|active| {
            active
                .session
                .borrow()
                .as_ref()
                .is_some_and(|session| session.track.axis() == Axis::Held)
        })
    }

    pub fn contact(&self) -> bool {
        self.active()
            .is_some_and(|active| active.session.borrow().is_some())
    }

    pub fn outside(&self, source: &Control) {
        if self
            .active()
            .is_some_and(|active| !active.contains_source(source))
        {
            self.cancel();
        }
    }
}

pub(super) fn claim(task: &Ref<TaskRecognizer>) {
    if let Some(shell) = shell(task) {
        let previous = shell.hub.active();
        if previous
            .as_ref()
            .is_some_and(|previous| !std::ptr::eq::<TaskRecognizer>(&**previous, &**task))
        {
            shell.hub.cancel();
        }
        *shell.hub.active.borrow_mut() = Some(task.downgrade());
    }
}

pub(super) fn shell(task: &TaskRecognizer) -> Option<Ref<ShellRecognizer>> {
    let target = task.target()?;
    for visual in target.get_visual_ancestors() {
        let Some(control) = visual.cast::<Control>() else {
            continue;
        };
        if control.name().as_deref() != Some("TodoApp") {
            continue;
        }
        for recognizer in control.gesture_recognizers().to_vec() {
            if let Some(shell) = recognizer.cast::<ShellRecognizer>() {
                return Some(shell);
            }
        }
    }
    None
}
