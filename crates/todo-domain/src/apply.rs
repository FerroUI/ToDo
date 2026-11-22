use crate::{Action, AppState, Changes, Date, DomainError, Id, Task};

impl AppState {
    /// Execute one command. Validation runs before mutation; a batch rolls back on any error.
    pub fn apply(&mut self, action: Action, today: Date) -> Result<Changes, DomainError> {
        let mut changes = match action {
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

    pub(crate) fn duplicate(&mut self, mut task: Task, today: Date) -> Task {
        task.id = self.allocate_id();
        task.created_on = today;
        task.updated_on = today;
        task.completed = false;
        task.completed_on = None;
        task.recurrence_generated = false;
        task.position = self
            .tasks
            .iter()
            .map(|task| task.position)
            .max()
            .unwrap_or(0)
            + 1;
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
