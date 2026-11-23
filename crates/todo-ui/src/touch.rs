//! Touch behavior shares the desktop controls, commands, and native input arbitration.
mod actions;
mod contacts;
mod context;
mod drag_session;
mod environment;
mod hub;
mod interruption;
mod pen;
mod refresh_feedback;
mod shell;
mod shell_input;
mod shell_lifetime;
mod sidebar;
mod sidebar_input;
mod surface;
mod task;
mod task_input;
mod tooltips;

pub(crate) use interruption::{cancel_active, cancel_pen, has_active, interrupted_contacts};
pub(crate) use shell::{attach_shell, shell_feedback};
pub(crate) use sidebar::attach_sidebar_source;
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
