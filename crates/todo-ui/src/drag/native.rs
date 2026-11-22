//! Lifetime of one armed mouse press or native in-process drag.
use super::{cancellation::Cancellation, feedback, registry, sidebar_feedback, DragItem};
use ferroui_base::input::IPointer;
use ferroui_base::WeakRef;
use ferroui_controls::{Button, Control};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use todo_state::{
    gestures::drag::{DragInput, DragOrigin},
    AppViewModel,
};

pub(super) struct NativeDrag {
    pub origin: DragOrigin,
    pub owner: Weak<AppViewModel>,
    pub root: WeakRef<Control>,
    pub pointer: Rc<dyn IPointer>,
    ended: Cell<bool>,
    dragging: Cell<bool>,
    cancellation: RefCell<Option<Cancellation>>,
}

impl NativeDrag {
    pub fn arm(
        source: &Button,
        vm: &Rc<AppViewModel>,
        item: DragItem,
        pointer: Rc<dyn IPointer>,
    ) -> Option<Rc<Self>> {
        let origin = DragOrigin::capture(vm, item, DragInput::Mouse)?;
        let root = source
            .get_self_and_visual_ancestors()
            .filter_map(|visual| visual.cast::<Control>())
            .find(|control| control.has_name("TodoApp"))?;
        registry::cancel_active(&root);
        let session = Rc::new(Self {
            origin,
            owner: Rc::downgrade(vm),
            root: root.downgrade(),
            pointer,
            ended: Cell::new(false),
            dragging: Cell::new(false),
            cancellation: RefCell::new(None),
        });
        registry::register(&session);
        *session.cancellation.borrow_mut() = Some(Cancellation::watch(&session, vm));
        session.synchronize();
        Some(session)
    }

    pub fn active(&self) -> bool {
        !self.ended.get() && self.origin.active()
    }

    pub fn valid(&self, vm: &Rc<AppViewModel>) -> bool {
        let valid = self.active()
            && self.owner.ptr_eq(&Rc::downgrade(vm))
            && self.origin.valid(vm)
            && self
                .root
                .upgrade()
                .is_some_and(|root| root.is_attached_to_visual_tree());
        if !valid {
            self.cancel();
        }
        valid
    }

    pub fn start(&self, vm: &Rc<AppViewModel>) -> bool {
        if !self.valid(vm) {
            return false;
        }
        self.dragging.set(true);
        self.synchronize();
        true
    }

    pub fn cancel(&self) -> bool {
        self.end(true)
    }

    /// Normal mouse-up disarms the observer without stealing the Button's release.
    /// An accepted Drop uses this before dispatch so a later callback cannot commit twice.
    pub fn finish(&self) -> bool {
        self.end(false)
    }

    fn end(&self, release_capture: bool) -> bool {
        if self.ended.replace(true) {
            return false;
        }
        let live = self.origin.finish();
        let cancellation = self.cancellation.borrow_mut().take();
        drop(cancellation);
        if release_capture {
            self.pointer.capture(None);
        }
        if self.dragging.get() {
            if let Some(root) = self.root.upgrade() {
                feedback::clear_root(&root);
                sidebar_feedback::clear_root(&root);
            }
        }
        self.synchronize();
        live
    }

    pub fn synchronize(&self) {
        if let (Some(vm), Some(root)) = (self.owner.upgrade(), self.root.upgrade()) {
            crate::shell::navigation::sync_host(&vm, Some(&root));
        }
    }

    pub fn matches_root(&self, root: &Control) -> bool {
        self.root.upgrade().is_some_and(|owner| {
            std::ptr::eq::<Control>(&*owner, root)
                || root.is_visual_ancestor_of(&owner)
                || owner.is_visual_ancestor_of(root)
        })
    }
}

impl Drop for NativeDrag {
    fn drop(&mut self) {
        self.end(false);
    }
}
