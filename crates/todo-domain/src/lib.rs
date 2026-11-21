//! Platform-independent task management. All mutations pass through [`AppState::apply`].
//! Dates are civil dates supplied by the host, so My Day never changes with UTC midnight.

mod action;
mod apply;
mod calendar;
mod lists;
mod model;
mod persistence;
mod query;
mod sample;
mod steps;
mod task_actions;

pub use action::*;
pub use calendar::*;
pub use chrono::NaiveDate as Date;
pub use model::*;
pub use query::*;

/// Entity identifiers are monotonic and local to one persisted database.
pub type Id = u64;

/// The permanent, non-deletable default Tasks list.
pub const TASKS_LIST: Id = 1;

/// Increment only with an explicit backwards-compatible migration.
pub const SCHEMA_VERSION: u32 = 1;
