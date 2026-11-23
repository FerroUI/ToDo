use crate::{Action, AppState, Changes, Date, DomainError, Step, Task, action::nonempty};

impl AppState {
    pub(crate) fn apply_step(
        &mut self,
        action: Action,
        today: Date,
    ) -> Result<Changes, DomainError> {
        match action {
            Action::AddStep { task_id, title } => {
                let title = nonempty(title)?;
                let index = self.task_index(task_id)?;
                let id = self.allocate_id();
                self.tasks[index].steps.push(Step {
                    id,
                    title,
                    completed: false,
                });
                self.tasks[index].updated_on = today;
                Ok(Changes::task(task_id))
            }
            Action::RenameStep {
                task_id,
                step_id,
                title,
            } => {
                let title = nonempty(title)?;
                let (index, step) = self.step_index(task_id, step_id)?;
                if self.tasks[index].steps[step].title == title {
                    return Ok(Changes::default());
                }
                self.tasks[index].steps[step].title = title;
                self.tasks[index].updated_on = today;
                Ok(Changes::task(task_id))
            }
            Action::ToggleStep { task_id, step_id } => {
                let (index, step) = self.step_index(task_id, step_id)?;
                self.tasks[index].steps[step].completed = !self.tasks[index].steps[step].completed;
                self.tasks[index].updated_on = today;
                Ok(Changes::task(task_id))
            }
            Action::DeleteStep { task_id, step_id } => {
                let (index, step) = self.step_index(task_id, step_id)?;
                self.tasks[index].steps.remove(step);
                self.tasks[index].updated_on = today;
                Ok(Changes::task(task_id))
            }
            Action::PromoteStep { task_id, step_id } => {
                let (index, step_index) = self.step_index(task_id, step_id)?;
                let position = self.next_task_position(self.tasks[index].list_id)?;
                let id = self.allocate_id();
                let step = self.tasks[index].steps.remove(step_index);
                let mut task = Task::new(id, self.tasks[index].list_id, step.title, today);
                task.position = position;
                task.completed = step.completed;
                task.completed_on = step.completed.then_some(today);
                self.tasks[index].updated_on = today;
                self.tasks.push(task);
                Ok(Changes {
                    changed_tasks: vec![task_id, id],
                    created_task: Some(id),
                    changed: true,
                    ..Changes::default()
                })
            }
            Action::AddAttachment {
                task_id,
                mut attachment,
            } => {
                let index = self.task_index(task_id)?;
                attachment.name = nonempty(attachment.name)?;
                if attachment.storage_key.trim().is_empty() {
                    return Err(DomainError::InvalidValue(
                        "Attachment has no storage key".into(),
                    ));
                }
                if attachment.size > 25 * 1024 * 1024 {
                    return Err(DomainError::InvalidValue(
                        "Attachments must be 25 MB or smaller".into(),
                    ));
                }
                attachment.id = self.allocate_id();
                self.tasks[index].attachments.push(attachment);
                self.tasks[index].updated_on = today;
                Ok(Changes::task(task_id))
            }
            Action::DeleteAttachment {
                task_id,
                attachment_id,
            } => {
                let index = self.task_index(task_id)?;
                let attachment_index = self.tasks[index]
                    .attachments
                    .iter()
                    .position(|attachment| attachment.id == attachment_id)
                    .ok_or(DomainError::NotFound("Attachment", attachment_id))?;
                self.tasks[index].attachments.remove(attachment_index);
                self.tasks[index].updated_on = today;
                Ok(Changes::task(task_id))
            }
            _ => Err(DomainError::InvalidValue("Unsupported step command".into())),
        }
    }

    fn step_index(
        &self,
        task_id: crate::Id,
        step_id: crate::Id,
    ) -> Result<(usize, usize), DomainError> {
        let index = self.task_index(task_id)?;
        let step = self.tasks[index]
            .steps
            .iter()
            .position(|step| step.id == step_id)
            .ok_or(DomainError::NotFound("Step", step_id))?;
        Ok((index, step))
    }
}
