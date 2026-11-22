use super::{shell::ShellRecognizer, surface::Surface};
use crate::drag::TouchDrag;
use ferroui_base::{input::IPointer, Point, Ref, WeakRef};
use ferroui_controls::Button;
use std::rc::{Rc, Weak};

/// A virtualized row can leave the tree during edge scrolling. The mounted shell owns
/// the operation and capture; its source visual is only a weak, optional decoration.
pub(super) struct DragSession {
    pub pointer: Rc<dyn IPointer>,
    operation: Option<TouchDrag>,
    source: Source,
}

enum Source {
    Task(Weak<Surface>),
    Sidebar(WeakRef<Button>, f64),
}

impl Source {
    fn dim(&self, active: bool) {
        match self {
            Self::Task(weak) => {
                if let Some(source) = weak.upgrade() {
                    source.dim(active);
                }
            }
            Self::Sidebar(weak, opacity) => {
                if let Some(source) = weak.upgrade() {
                    source.set_opacity(if active { 0.65 } else { *opacity });
                }
            }
        }
    }
}

impl DragSession {
    pub fn update(&self, point: Point) {
        if let Some(operation) = &self.operation {
            operation.update(point);
        }
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
    pub fn begin_drag(
        &self,
        pointer: Rc<dyn IPointer>,
        operation: TouchDrag,
        source: &Rc<Surface>,
    ) {
        self.start_drag(pointer, operation, Source::Task(Rc::downgrade(source)));
    }

    pub fn begin_sidebar_drag(
        &self,
        pointer: Rc<dyn IPointer>,
        operation: TouchDrag,
        source: &Ref<Button>,
    ) {
        self.start_drag(
            pointer,
            operation,
            Source::Sidebar(source.downgrade(), source.opacity()),
        );
    }

    fn start_drag(&self, pointer: Rc<dyn IPointer>, operation: TouchDrag, source: Source) {
        self.cancel();
        let message = operation.label();
        source.dim(true);
        *self.drag.borrow_mut() = Some(DragSession {
            pointer: pointer.clone(),
            operation: Some(operation),
            source,
        });
        self.capture(&pointer);
        self.feedback(Some(message));
    }
}
