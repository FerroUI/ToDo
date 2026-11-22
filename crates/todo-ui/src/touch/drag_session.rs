use super::{shell::ShellRecognizer, surface::Surface};
use crate::drag::TouchDrag;
use ferroui_base::{Point, input::IPointer};
use std::rc::{Rc, Weak};

/// A virtualized row can leave the tree during edge scrolling. The mounted shell owns
/// the operation and capture; its source visual is only a weak, optional decoration.
pub(super) struct DragSession {
    pub pointer: Rc<dyn IPointer>,
    operation: Option<TouchDrag>,
    source: Weak<Surface>,
}

impl DragSession {
    pub fn update(&self, point: Point) {
        if let Some(operation) = &self.operation {
            operation.update(point);
        }
    }

    pub fn finish(mut self, point: Point) {
        if let Some(source) = self.source.upgrade() {
            source.dim(false);
        }
        if let Some(operation) = self.operation.take() {
            operation.finish(point);
        }
    }
}

impl Drop for DragSession {
    fn drop(&mut self) {
        if let Some(source) = self.source.upgrade() {
            source.dim(false);
        }
    }
}

impl ShellRecognizer {
    pub fn begin_drag(
        &self,
        pointer: Rc<dyn IPointer>,
        operation: TouchDrag,
        source: &Rc<Surface>,
    ) {
        self.cancel();
        *self.drag.borrow_mut() = Some(DragSession {
            pointer: pointer.clone(),
            operation: Some(operation),
            source: Rc::downgrade(source),
        });
        self.capture(&pointer);
        source.dim(true);
        self.feedback(Some("Move task"));
    }
}
