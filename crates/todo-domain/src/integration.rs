use crate::{Action, AppState, DomainError};
use serde::{Deserialize, Serialize};

/// A source record remains distinct from a personal To Do task.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalTask {
    pub provider: String,
    pub id: String,
    pub url: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedResource {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub application_name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub web_url: String,
    #[serde(default)]
    pub external_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudList {
    pub is_owner: bool,
    pub is_shared: bool,
    pub well_known_name: String,
}

impl AppState {
    /// Enforce service capabilities below every UI, keyboard and batch command.
    pub(crate) fn validate_integration_action(&self, action: &Action) -> Result<(), DomainError> {
        match action {
            Action::MoveTask { id, .. } | Action::ReorderTask { id, .. }
                if self.task(*id).is_some_and(|task| !task.can_move()) =>
            {
                return Err(DomainError::InvalidValue(
                    "This task stays in its source list. Duplicate it to create an independent task you can move.".into(),
                ));
            }
            Action::DeleteTask(id) if self.task(*id).is_some_and(|task| !task.can_delete()) => {
                return Err(DomainError::InvalidValue(
                    "Open this task in Planner to delete it.".into(),
                ));
            }
            _ => {}
        }
        let task = match action {
            Action::AssignTask { id, .. }
            | Action::SetReminder { id, .. }
            | Action::SetRecurrence { id, .. }
            | Action::SetTags { id, .. } => Some(*id),
            Action::AddAttachment { task_id, .. }
            | Action::DeleteAttachment { task_id, .. }
            | Action::PromoteStep { task_id, .. } => Some(*task_id),
            _ => None,
        };
        if task
            .and_then(|id| self.task(id))
            .is_some_and(|task| task.is_planner())
        {
            return Err(DomainError::InvalidValue("Open this task in Planner to change its plan, assignment, files, or other service settings.".into()));
        }
        let list = match action {
            Action::RenameList { id, .. } | Action::DeleteList(id) => Some(*id),
            _ => None,
        };
        if list
            .and_then(|id| self.list(id))
            .and_then(|list| list.cloud.as_ref())
            .is_some_and(|cloud| !cloud.is_owner || cloud.well_known_name != "none")
        {
            return Err(DomainError::InvalidValue(
                "This Microsoft list cannot be renamed or deleted from this account.".into(),
            ));
        }
        Ok(())
    }
}

pub(crate) fn valid_source_url(value: &str) -> bool {
    (value.starts_with("https://") || value.starts_with("http://"))
        && !value.chars().any(char::is_control)
}
