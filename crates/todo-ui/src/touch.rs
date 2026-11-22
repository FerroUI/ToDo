//! Touch behavior shares the desktop controls, commands, and native input arbitration.
mod actions;
mod drag_session;
mod hub;
mod shell;
mod shell_input;
mod shell_lifetime;
mod surface;
mod task;
mod task_input;

pub(crate) use shell::{attach_shell, shell_feedback};
pub(crate) use surface::task_surface;

use ferroui_controls::Control;
use todo_domain::Id;

#[derive(Clone, Copy, PartialEq, Eq)]
struct TaskSurfaceId(Id);

pub(crate) fn task_id(control: &Control) -> Option<Id> {
    control
        .tag()
        .and_then(|tag| tag.downcast_ref::<TaskSurfaceId>().map(|id| id.0))
}
