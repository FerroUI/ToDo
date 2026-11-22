use super::{shell::ShellRecognizer, surface::Surface, task, task_id};
use crate::drag::{sidebar_identity, TouchDrag};
use ferroui_base::{input::IPointer, Point, Ref, WeakRef};
use ferroui_controls::Button;
use std::rc::{Rc, Weak};
use todo_domain::{Id, SidebarItem};
use todo_state::{gestures::TouchTrack, AppViewModel};

/// A virtualized row can leave the tree during edge scrolling. The mounted shell owns
/// the operation and capture; its source visual is only a weak, optional decoration.
pub(super) struct DragSession {
    pub pointer: Rc<dyn IPointer>,
    operation: Option<TouchDrag>,
    source: Source,
    pending: Option<(Point, TouchTrack)>,
}

enum Source {
    Task(Weak<Surface>, Id),
    Sidebar(WeakRef<Button>, SidebarItem, f64),
}

impl Source {
    fn dim(&self, active: bool) {
        match self {
            Self::Task(weak, id) => {
                if let Some(source) = weak.upgrade().filter(|source| {
                    source.root.upgrade().is_some_and(|root| {
                        root.is_attached_to_visual_tree() && task_id(&root) == Some(*id)
                    })
                }) {
                    source.dim(active);
                }
            }
            Self::Sidebar(weak, item, opacity) => {
                if let Some(source) = weak
                    .upgrade()
                    .filter(|source| sidebar_identity(source) == Some(*item))
                {
                    source.set_opacity(if active { 0.65 } else { *opacity });
                }
            }
        }
    }
}

impl DragSession {
    pub fn valid(&self, vm: &AppViewModel) -> bool {
        if let Some(operation) = &self.operation {
            return operation.valid(vm);
        }
        // A smart/sorted-view hold owns capture for selection without a reorder
        // operation. Removing its task must still interrupt that held contact.
        match &self.source {
            Source::Task(_, id) => vm.state().task(*id).is_some(),
            Source::Sidebar(_, _, _) => false,
        }
    }

    pub fn update(&mut self, point: Point) -> Option<&'static str> {
        if let Some((start, track)) = &mut self.pending {
            track.update(point.x - start.x, point.y - start.y);
            if !track.moved_after_hold() {
                return None;
            }
            self.pending.take();
            if let Some(operation) = &mut self.operation {
                if operation.activate(point) {
                    self.source.dim(true);
                    return Some(operation.label());
                }
                return None;
            }
        }
        if let Some(operation) = &self.operation {
            operation.update(point);
        }
        None
    }

    pub fn finish(mut self, point: Point) {
        self.source.dim(false);
        if let Some(operation) = self.operation.take() {
            operation.finish(point);
        }
    }
}

impl Drop for DragSession {
    fn drop(&mut self) {
        self.source.dim(false);
    }
}

impl ShellRecognizer {
    pub fn begin_task_hold(&self, session: task::Session, source: &Rc<Surface>) {
        let operation = self.vm.upgrade().and_then(|vm| {
            source
                .root
                .upgrade()
                .and_then(|root| TouchDrag::pending_task(&root, &vm, session.id, session.start))
        });
        // Sorted/smart views still own the held contact, even though they cannot reorder.
        // The shell must capture before selection's item replacements detach the source.
        self.start_drag(DragSession {
            pointer: session.pointer,
            operation,
            source: Source::Task(Rc::downgrade(source), session.id),
            pending: Some((session.start, session.track)),
        });
    }

    pub fn begin_sidebar_drag(
        &self,
        pointer: Rc<dyn IPointer>,
        operation: TouchDrag,
        source: &Ref<Button>,
    ) {
        let Some(item) = sidebar_identity(source) else {
            return;
        };
        self.start_drag(DragSession {
            pointer,
            operation: Some(operation),
            source: Source::Sidebar(source.downgrade(), item, source.opacity()),
            pending: None,
        });
    }

    fn start_drag(&self, drag: DragSession) {
        self.cancel();
        let message = if drag.pending.is_none() {
            drag.source.dim(true);
            drag.operation.as_ref().map(TouchDrag::label)
        } else {
            None
        };
        let pointer = drag.pointer.clone();
        *self.drag.borrow_mut() = Some(drag);
        self.capture(&pointer);
        self.feedback(message);
        self.sync_back();
    }
}
