use super::{shell::ShellRecognizer, sidebar::SidebarRecognizer, task::TaskRecognizer};
use ferroui_base::input::gesture_recognizers::GestureRecognizer;
use ferroui_base::input::{IPointer, InputElement};
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::Control;
use std::cell::RefCell;
use std::rc::Rc;
use todo_state::gestures::Axis;

/// One coordinator belongs to one mounted shell. It never retains a row or application strongly.
#[derive(Default)]
pub(super) struct Hub {
    active: RefCell<Option<WeakRef<GestureRecognizer>>>,
}

impl Hub {
    fn active(&self) -> Option<Ref<GestureRecognizer>> {
        self.active.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    pub fn has_active(&self) -> bool {
        self.active().is_some_and(|active| {
            if let Some(task) = active.cast::<TaskRecognizer>() {
                task.active()
            } else if let Some(sidebar) = active.cast::<SidebarRecognizer>() {
                sidebar.session.borrow().is_some()
            } else {
                false
            }
        })
    }

    pub fn cancel(&self) -> bool {
        let active = self
            .active
            .borrow_mut()
            .take()
            .and_then(|weak| weak.upgrade());
        if let Some(task) = active
            .as_ref()
            .and_then(|active| active.cast::<TaskRecognizer>())
        {
            let handled = task.active();
            task.cancel();
            handled
        } else if let Some(sidebar) = active.and_then(|active| active.cast::<SidebarRecognizer>()) {
            let handled = sidebar.session.borrow().is_some();
            sidebar.cancel();
            handled
        } else {
            false
        }
    }

    pub fn held(&self) -> bool {
        self.active().is_some_and(|active| {
            if let Some(task) = active.cast::<TaskRecognizer>() {
                task.session
                    .borrow()
                    .as_ref()
                    .is_some_and(|session| session.track.axis() == Axis::Held)
            } else if let Some(sidebar) = active.cast::<SidebarRecognizer>() {
                sidebar
                    .session
                    .borrow()
                    .as_ref()
                    .is_some_and(|session| session.track.axis() == Axis::Held)
            } else {
                false
            }
        })
    }

    pub fn contact(&self) -> bool {
        self.active().is_some_and(|active| {
            if let Some(task) = active.cast::<TaskRecognizer>() {
                task.session.borrow().is_some()
            } else if let Some(sidebar) = active.cast::<SidebarRecognizer>() {
                sidebar.session.borrow().is_some()
            } else {
                false
            }
        })
    }

    pub fn pointer(&self) -> Option<Rc<dyn IPointer>> {
        let active = self.active()?;
        if let Some(task) = active.cast::<TaskRecognizer>() {
            task.session
                .borrow()
                .as_ref()
                .map(|session| session.pointer.clone())
        } else if let Some(sidebar) = active.cast::<SidebarRecognizer>() {
            sidebar
                .session
                .borrow()
                .as_ref()
                .map(|session| session.pointer.clone())
        } else {
            None
        }
    }

    pub fn outside(&self, source: &Control) {
        let source: &InputElement = source;
        if self
            .active()
            .and_then(|active| active.target())
            .is_some_and(|target| {
                !std::ptr::eq(&*target, source) && !target.is_visual_ancestor_of(source)
            })
        {
            self.cancel();
        }
    }
}

pub(super) fn claim(recognizer: &GestureRecognizer) {
    if let Some(shell) = shell(recognizer) {
        if shell
            .hub
            .active()
            .is_some_and(|previous| !std::ptr::eq(&*previous, recognizer))
        {
            shell.hub.cancel();
        }
        *shell.hub.active.borrow_mut() = Some(recognizer.to_ref().downgrade());
        shell.sync_back();
    }
}

pub(super) fn synchronize(recognizer: &GestureRecognizer) {
    if let Some(shell) = shell(recognizer) {
        shell.sync_back();
    }
}

pub(super) fn shell(recognizer: &GestureRecognizer) -> Option<Ref<ShellRecognizer>> {
    let target = recognizer.target()?;
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
