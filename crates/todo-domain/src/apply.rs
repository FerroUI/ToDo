use crate::{Action, AppState, Changes, Date, DomainError, Id, Task};

impl AppState {
    /// Execute one command. Validation runs before mutation; a batch rolls back on any error.
    pub fn apply(&mut self, action: Action, today: Date) -> Result<Changes, DomainError> {
        self.validate_integration_action(&action)?;
        let mut changes = if let Some(list_id) = self.task_order_needs_repair(&action, today) {
            // Rare saturated imports remain editable. A rejected task/step/title
            // leaves both order keys and the allocation cursor untouched.
            let mut transaction = self.clone();
            let mut changes = transaction.compact_task_order(list_id, today);
            changes.merge(transaction.apply(action, today)?);
            *self = transaction;
            changes
        } else {
            match action {
                Action::Batch(actions) => {
                    let mut transaction = self.clone();
                    let mut changes = Changes::default();
                    for action in actions {
                        changes.merge(transaction.apply(action, today)?);
                    }
                    *self = transaction;
                    changes
                }
                Action::SetSettings(settings) => {
                    let changed = self.settings != settings;
                    self.settings = settings;
                    Changes {
                        changed,
                        settings_changed: changed,
                        ..Changes::default()
                    }
                }
                Action::AddList { .. }
                | Action::RenameList { .. }
                | Action::DeleteList(_)
                | Action::DuplicateList(_)
                | Action::SetListTheme { .. }
                | Action::SetListSort { .. }
                | Action::SetListSortDirection { .. }
                | Action::SetListShowCompleted { .. }
                | Action::MoveList { .. }
                | Action::ReorderList { .. }
                | Action::AddGroup { .. }
                | Action::RenameGroup { .. }
                | Action::ReorderGroup { .. }
                | Action::ReorderSidebar { .. }
                | Action::DeleteGroup(_)
                | Action::ToggleGroup(_) => self.apply_list(action, today)?,
                Action::AddStep { .. }
                | Action::RenameStep { .. }
                | Action::ToggleStep { .. }
                | Action::DeleteStep { .. }
                | Action::PromoteStep { .. }
                | Action::AddAttachment { .. }
                | Action::DeleteAttachment { .. } => self.apply_step(action, today)?,
                _ => self.apply_task(action, today)?,
            }
        };
        changes.changed_tasks.sort_unstable();
        changes.changed_tasks.dedup();
        changes.deleted_tasks.sort_unstable();
        changes.deleted_tasks.dedup();
        Ok(changes)
    }

    pub(crate) fn task_index(&self, id: Id) -> Result<usize, DomainError> {
        self.tasks
            .iter()
            .position(|task| task.id == id)
            .ok_or(DomainError::NotFound("Task", id))
    }

    pub(crate) fn list_index(&self, id: Id) -> Result<usize, DomainError> {
        self.lists
            .iter()
            .position(|list| list.id == id)
            .ok_or(DomainError::NotFound("List", id))
    }

    /// Validate append placement before allocating IDs or changing a source task.
    pub(crate) fn next_task_position(&self, list_id: Id) -> Result<i64, DomainError> {
        self.tasks
            .iter()
            .filter(|task| task.list_id == list_id && task.can_move())
            .map(|task| task.position)
            .max()
            .unwrap_or(-1)
            .checked_add(1)
            .ok_or_else(|| {
                DomainError::InvalidValue("The destination task order is out of range".into())
            })
    }

    pub(crate) fn duplicate(&mut self, mut task: Task, today: Date, position: i64) -> Task {
        task.external = None;
        task.flagged_email = false;
        task.assigned_to = None;
        task.id = self.allocate_id();
        task.created_on = today;
        task.updated_on = today;
        task.completed = false;
        task.completed_on = None;
        task.recurrence_generated = false;
        task.position = position;
        for step in &mut task.steps {
            step.id = self.allocate_id();
            step.completed = false;
        }
        for attachment in &mut task.attachments {
            attachment.id = self.allocate_id();
        }
        task
    }
}
