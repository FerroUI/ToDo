use crate::{AppState, Changes, Date, DomainError, Task, TaskEntry, View, action::nonempty};

impl AppState {
    pub(crate) fn add_task_in_view(
        &mut self,
        entry: TaskEntry,
        view: View,
        today: Date,
    ) -> Result<Changes, DomainError> {
        let title = nonempty(entry.title)?;
        if entry
            .reminder
            .as_ref()
            .is_some_and(|value| value.minutes_after_midnight >= 1440)
        {
            return Err(DomainError::InvalidValue(
                "Reminder time must be within one day".into(),
            ));
        }
        let list_id = view.destination_list();
        self.list_index(list_id)?;
        let id = self.allocate_id();
        let mut task = Task::new(id, list_id, title, today);
        task.due_date = entry.due_date;
        task.reminder = entry.reminder;
        match view {
            View::MyDay => task.my_day = Some(today),
            View::Important => task.important = true,
            View::Planned if task.due_date.is_none() && task.reminder.is_none() => {
                task.due_date = Some(today)
            }
            View::AssignedToMe => task.assigned_to = Some("me".into()),
            _ => {}
        }
        self.tasks.push(task);
        Ok(Changes {
            created_task: Some(id),
            ..Changes::task(id)
        })
    }
}
