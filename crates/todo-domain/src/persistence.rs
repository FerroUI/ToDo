use crate::{AppState, DomainError, SCHEMA_VERSION, TASKS_LIST, lists::valid_color};
use std::collections::HashSet;

const MAX_BACKUP_BYTES: usize = 32 * 1024 * 1024;

impl AppState {
    pub fn to_json(&self) -> Result<String, DomainError> {
        serde_json::to_string(self).map_err(|error| DomainError::InvalidData(error.to_string()))
    }

    pub fn to_json_pretty(&self) -> Result<String, DomainError> {
        serde_json::to_string_pretty(self)
            .map_err(|error| DomainError::InvalidData(error.to_string()))
    }

    /// Parse into an isolated state and validate every foreign key before replacing live data.
    /// Unknown future versions are rejected so an older client cannot destroy newer fields.
    pub fn from_json(json: &str) -> Result<Self, DomainError> {
        if json.len() > MAX_BACKUP_BYTES {
            return Err(DomainError::InvalidData("Backup exceeds 32 MB".into()));
        }
        let mut state: Self = serde_json::from_str(json)
            .map_err(|error| DomainError::InvalidData(format!("Cannot read backup: {error}")))?;
        if state.schema_version > SCHEMA_VERSION {
            return Err(DomainError::FutureSchema(state.schema_version));
        }
        if state.schema_version != SCHEMA_VERSION {
            return Err(DomainError::InvalidData("Unsupported backup format".into()));
        }
        // Navigation preference may retain IDs deleted by an older or remote client.
        // Repair only this optional projection; foreign keys and entity data stay strict.
        state.repair_sidebar_order();
        state.validate()?;
        let maximum = state.all_ids().max().unwrap_or(TASKS_LIST);
        // Older saves may have a stale allocation cursor. Repair the cursor, never entity data.
        state.next_id = state.next_id.max(maximum + 1);
        Ok(state)
    }

    pub fn validate(&self) -> Result<(), DomainError> {
        let invalid = |message: &str| DomainError::InvalidData(message.into());
        if self
            .settings
            .smart_colors
            .values()
            .any(|color| !valid_color(color))
        {
            return Err(invalid("A smart list theme color is invalid"));
        }
        let mut ids = HashSet::new();
        for id in self.all_ids() {
            if id == 0 || id >= i64::MAX as u64 {
                return Err(invalid("Entity identifier is out of range"));
            }
            if !ids.insert(id) {
                return Err(invalid("Backup contains duplicate entity identifiers"));
            }
        }
        if self.next_id == 0 || self.next_id >= i64::MAX as u64 {
            return Err(invalid("Next entity identifier is out of range"));
        }
        let lists: HashSet<_> = self.lists.iter().map(|list| list.id).collect();
        let groups: HashSet<_> = self.groups.iter().map(|group| group.id).collect();
        if !lists.contains(&TASKS_LIST) {
            return Err(invalid("Backup has no default Tasks list"));
        }
        for group in &self.groups {
            if group.name.trim().is_empty() {
                return Err(invalid("A group name is empty"));
            }
        }
        for list in &self.lists {
            if list.name.trim().is_empty() {
                return Err(invalid("A list name is empty"));
            }
            if !valid_color(&list.color) {
                return Err(invalid("A list theme color is invalid"));
            }
            if list.group_id.is_some_and(|id| !groups.contains(&id)) {
                return Err(invalid("A list refers to a missing group"));
            }
            if list.id == TASKS_LIST && list.group_id.is_some() {
                return Err(invalid("The default Tasks list cannot belong to a group"));
            }
        }
        self.validate_sidebar_order()?;
        for task in &self.tasks {
            if task.title.trim().is_empty() {
                return Err(invalid("A task title is empty"));
            }
            if !lists.contains(&task.list_id) {
                return Err(invalid("A task refers to a missing list"));
            }
            if task
                .reminder
                .as_ref()
                .is_some_and(|reminder| reminder.minutes_after_midnight >= 1440)
            {
                return Err(invalid("A reminder time is invalid"));
            }
            if task
                .recurrence
                .as_ref()
                .is_some_and(|recurrence| recurrence.every == 0)
            {
                return Err(invalid("A repeat interval is zero"));
            }
            for step in &task.steps {
                if step.title.trim().is_empty() {
                    return Err(invalid("A step title is empty"));
                }
            }
            for attachment in &task.attachments {
                if attachment.name.trim().is_empty()
                    || attachment.storage_key.trim().is_empty()
                    || attachment.size > 25 * 1024 * 1024
                {
                    return Err(invalid("Attachment metadata is invalid"));
                }
            }
        }
        Ok(())
    }

    fn all_ids(&self) -> impl Iterator<Item = crate::Id> + '_ {
        self.lists
            .iter()
            .map(|list| list.id)
            .chain(self.groups.iter().map(|group| group.id))
            .chain(self.tasks.iter().map(|task| task.id))
            .chain(
                self.tasks
                    .iter()
                    .flat_map(|task| task.steps.iter().map(|step| step.id)),
            )
            .chain(
                self.tasks
                    .iter()
                    .flat_map(|task| task.attachments.iter().map(|attachment| attachment.id)),
            )
    }
}
