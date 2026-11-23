//! A target lease survives synchronous native data reads, which may pump events.
use super::show;
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::Control;
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use todo_state::AppViewModel;

thread_local! {
    static TARGETS: RefCell<Vec<Weak<Target>>> = const { RefCell::new(Vec::new()) };
}

#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Hover,
    Reading,
    Finished,
}

pub(super) struct Target {
    control: WeakRef<Control>,
    owner: Weak<AppViewModel>,
    current: RefCell<Option<Rc<Cell<Phase>>>>,
}

impl Target {
    pub fn new(control: &Ref<Control>, vm: &Rc<AppViewModel>) -> Rc<Self> {
        let target = Rc::new(Self {
            control: control.downgrade(),
            owner: Rc::downgrade(vm),
            current: RefCell::default(),
        });
        TARGETS.with(|targets| {
            let mut targets = targets.borrow_mut();
            targets.retain(|target| target.strong_count() > 0);
            targets.push(Rc::downgrade(&target));
        });
        target
    }

    pub fn enter(&self) {
        // Replacing a hover invalidates any older read without briefly removing
        // the host's Back capability between consecutive native DragEnter events.
        if let Some(previous) = self.current.replace(Some(Rc::new(Cell::new(Phase::Hover)))) {
            previous.set(Phase::Finished);
        }
    }

    pub fn hovering(&self) -> bool {
        self.current
            .borrow()
            .as_ref()
            .is_some_and(|lease| lease.get() == Phase::Hover)
    }

    pub fn begin_drop(self: &Rc<Self>) -> Option<DropLease> {
        let lease = self.current.borrow().clone()?;
        if lease.get() != Phase::Hover {
            return None;
        }
        lease.set(Phase::Reading);
        // Keep the lease registered until materialization and focus callbacks finish.
        // Escape or host cancellation must still invalidate a reentrant GetData.
        Some(DropLease {
            target: self.clone(),
            lease,
        })
    }

    pub fn cancel(&self) -> bool {
        let lease = self.current.borrow_mut().take();
        let Some(lease) = lease else { return false };
        lease.set(Phase::Finished);
        if let Some(control) = self.control.upgrade() {
            show(&control, false);
        }
        self.synchronize();
        true
    }

    fn active(&self) -> bool {
        self.current.borrow().is_some()
    }

    fn matches_root(&self, root: &Control) -> bool {
        self.control.upgrade().is_some_and(|control| {
            std::ptr::eq::<Control>(&*control, root)
                || root.is_visual_ancestor_of(&control)
                || control.is_visual_ancestor_of(root)
        })
    }

    pub fn synchronize(&self) {
        let (Some(control), Some(vm)) = (self.control.upgrade(), self.owner.upgrade()) else {
            return;
        };
        if let Some(root) = control
            .get_self_and_visual_ancestors()
            .filter_map(|visual| visual.cast::<Control>())
            .find(|control| control.has_name("TodoApp"))
        {
            crate::shell::navigation::sync_host(&vm, Some(&root));
        }
    }
}

pub(super) struct DropLease {
    target: Rc<Target>,
    lease: Rc<Cell<Phase>>,
}

impl DropLease {
    pub fn active(&self) -> bool {
        self.lease.get() == Phase::Reading
    }

    pub fn commit(&self, apply: impl FnOnce() -> Result<(), String>) -> Result<bool, String> {
        if !self.active() {
            return Ok(false);
        }
        // Reentrant notifications can cancel after the transaction commits. The
        // acknowledged copy follows its result, not the lease's later state.
        apply().map(|()| true)
    }
}

impl Drop for DropLease {
    fn drop(&mut self) {
        let current = self
            .target
            .current
            .borrow()
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, &self.lease));
        self.lease.set(Phase::Finished);
        // A provider may have pumped a fresh DragEnter. Finishing an older read
        // cannot clear that new operation's lease or feedback.
        if current {
            self.target.cancel();
        }
    }
}

fn targets(root: &Control) -> Vec<Rc<Target>> {
    TARGETS.with(|targets| {
        targets
            .borrow()
            .iter()
            .filter_map(Weak::upgrade)
            .filter(|target| target.active() && target.matches_root(root))
            .collect()
    })
}

pub(crate) fn has_active(root: &Control) -> bool {
    !targets(root).is_empty()
}

pub(crate) fn cancel_active(root: &Control) -> bool {
    // Visibility and host notifications can reenter; release the registry first.
    let targets = targets(root);
    let canceled = !targets.is_empty();
    for target in targets {
        target.cancel();
    }
    canceled
}
