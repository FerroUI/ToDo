use crate::{Action, AppState, Changes, Date, DomainError, Id, Task, action::nonempty};

impl AppState {
    pub(crate) fn apply_task(
        &mut self,
        action: Action,
        today: Date,
    ) -> Result<Changes, DomainError> {
        match action {
            Action::AddTaskInView { title, view } => {
                self.add_task_in_view(crate::TaskEntry::plain(title), view, today)
            }
            Action::AddTaskEntry { entry, view } => self.add_task_in_view(entry, view, today),
            Action::AddTask { title, list_id } => {
                let title = nonempty(title)?;
                self.list_index(list_id)?;
                let id = self.allocate_id();
                self.tasks.push(Task::new(id, list_id, title, today));
                Ok(Changes {
                    created_task: Some(id),
                    ..Changes::task(id)
                })
            }
            Action::RenameTask { id, title } => {
                let title = nonempty(title)?;
                self.edit_task(id, today, |task| task.title = title)
            }
            Action::SetNotes { id, notes } => self.edit_task(id, today, |task| task.notes = notes),
            Action::ToggleCompleted(id) => {
                let completed = !self.tasks[self.task_index(id)?].completed;
                self.complete_task(id, completed, today)
            }
            Action::SetCompleted { id, completed } => self.complete_task(id, completed, today),
            Action::ToggleImportant(id) => {
                self.edit_task(id, today, |task| task.important = !task.important)
            }
            Action::SetImportant { id, important } => {
                self.edit_task(id, today, |task| task.important = important)
            }
            Action::SetMyDay { id, day } => self.edit_task(id, today, |task| {
                task.my_day = day;
                task.my_day_excluded = if day.is_none() { Some(today) } else { None };
            }),
            Action::SetDueDate { id, date } => self.edit_task(id, today, |task| {
                if task.due_date == date {
                    return;
                }
                task.due_date = date;
                if let (Some(recurrence), Some(date)) = (&mut task.recurrence, date) {
                    recurrence.anchor = Some(date);
                }
            }),
            Action::SetReminder { id, mut reminder } => {
                if reminder
                    .as_ref()
                    .is_some_and(|value| value.minutes_after_midnight >= 1440)
                {
                    return Err(DomainError::InvalidValue(
                        "Reminder time must be within one day".into(),
                    ));
                }
                self.edit_task(id, today, |task| {
                    if let (Some(existing), Some(value)) = (&task.reminder, &mut reminder) {
                        if existing.date == value.date
                            && existing.minutes_after_midnight == value.minutes_after_midnight
                        {
                            // An unchanged dialog save must not rearm a delivered
                            // reminder. Explicit delivery acknowledgement still wins.
                            value.notified |= existing.notified;
                        }
                    }
                    task.reminder = reminder;
                })
            }
            Action::SetRecurrence { id, mut recurrence } => {
                if recurrence.as_ref().is_some_and(|value| value.every == 0) {
                    return Err(DomainError::InvalidValue(
                        "Repeat interval must be at least one".into(),
                    ));
                }
                let index = self.task_index(id)?;
                if let Some(value) = &mut recurrence {
                    if value.anchor.is_none() {
                        let task = &self.tasks[index];
                        // Saving an unchanged rule must not shift a month-end
                        // series from its original anchor to a clamped date.
                        value.anchor = match task.recurrence.as_ref().filter(|existing| {
                            existing.unit == value.unit && existing.every == value.every
                        }) {
                            Some(existing) => existing.anchor,
                            None => Some(task.due_date.unwrap_or(today)),
                        };
                    }
                }
                self.edit_task(id, today, |task| {
                    task.recurrence = recurrence;
                })
            }
            Action::SetTags { id, tags } => {
                let mut tags: Vec<String> = tags
                    .into_iter()
                    .map(|tag| tag.trim().to_owned())
                    .filter(|tag| !tag.is_empty())
                    .collect();
                tags.sort_by_key(|tag| tag.to_lowercase());
                tags.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
                self.edit_task(id, today, |task| task.tags = tags)
            }
            Action::AssignTask { id, assignee } => self.edit_task(id, today, |task| {
                task.assigned_to = assignee.filter(|value| !value.trim().is_empty());
            }),
            Action::MoveTask { id, list_id } => {
                self.list_index(list_id)?;
                if self.tasks[self.task_index(id)?].list_id == list_id {
                    return Ok(Changes::default());
                }
                let position = self
                    .tasks
                    .iter()
                    .filter(|task| task.list_id == list_id)
                    .map(|task| task.position)
                    .max()
                    .unwrap_or(-1)
                    .checked_add(1)
                    .ok_or_else(|| {
                        DomainError::InvalidValue(
                            "The destination task order is out of range".into(),
                        )
                    })?;
                self.edit_task(id, today, |task| {
                    task.list_id = list_id;
                    task.position = position;
                })
            }
            Action::DuplicateTask(id) => {
                let original = self.tasks[self.task_index(id)?].clone();
                let task = self.duplicate(original, today);
                let id = task.id;
                self.tasks.push(task);
                Ok(Changes {
                    created_task: Some(id),
                    ..Changes::task(id)
                })
            }
            Action::DeleteTask(id) => {
                let index = self.task_index(id)?;
                self.tasks.remove(index);
                Ok(Changes {
                    changed: true,
                    deleted_tasks: vec![id],
                    ..Changes::default()
                })
            }
            Action::DeleteCompleted { list_id } => {
                if let Some(id) = list_id {
                    self.list_index(id)?;
                }
                let ids: Vec<Id> = self
                    .tasks
                    .iter()
                    .filter(|task| {
                        task.can_delete()
                            && task.completed
                            && list_id.is_none_or(|id| task.list_id == id)
                    })
                    .map(|task| task.id)
                    .collect();
                self.tasks.retain(|task| {
                    !task.can_delete()
                        || !(task.completed && list_id.is_none_or(|id| task.list_id == id))
                });
                Ok(Changes {
                    changed: !ids.is_empty(),
                    deleted_tasks: ids,
                    ..Changes::default()
                })
            }
            Action::ReorderTask { id, before } => self.reorder_task(id, before, today),
            _ => Err(DomainError::InvalidValue("Unsupported task command".into())),
        }
    }

    fn edit_task(
        &mut self,
        id: Id,
        today: Date,
        edit: impl FnOnce(&mut Task),
    ) -> Result<Changes, DomainError> {
        let index = self.task_index(id)?;
        let previous = self.tasks[index].clone();
        edit(&mut self.tasks[index]);
        if self.tasks[index] == previous {
            return Ok(Changes::default());
        }
        self.tasks[index].updated_on = today;
        Ok(Changes::task(id))
    }

    fn complete_task(
        &mut self,
        id: Id,
        completed: bool,
        today: Date,
    ) -> Result<Changes, DomainError> {
        let index = self.task_index(id)?;
        if self.tasks[index].completed == completed {
            return Ok(Changes::default());
        }
        let task = &mut self.tasks[index];
        task.completed = completed;
        task.completed_on = completed.then_some(today);
        task.updated_on = today;
        let mut changes = Changes::task(id);
        if completed && !task.recurrence_generated {
            if let Some(recurrence) = task.recurrence.clone() {
                if let Some(next) = recurrence.next_after(task.due_date.unwrap_or(today), today) {
                    task.recurrence_generated = true;
                    let original = task.clone();
                    let old_due = original.due_date.unwrap_or(today);
                    let mut next_task = self.duplicate(original, today);
                    next_task.my_day = None;
                    next_task.my_day_excluded = None;
                    next_task.due_date = Some(next);
                    if let Some(reminder) = &mut next_task.reminder {
                        reminder.date = next
                            .checked_add_signed(reminder.date - old_due)
                            .unwrap_or(next);
                        reminder.notified = false;
                    }
                    changes.changed_tasks.push(next_task.id);
                    changes.created_task = Some(next_task.id);
                    self.tasks.push(next_task);
                }
            }
        }
        Ok(changes)
    }

    fn reorder_task(
        &mut self,
        id: Id,
        before: Option<Id>,
        today: Date,
    ) -> Result<Changes, DomainError> {
        let list_id = self.tasks[self.task_index(id)?].list_id;
        if before == Some(id) {
            return Ok(Changes::default());
        }
        if let Some(before_id) = before {
            let target = &self.tasks[self.task_index(before_id)?];
            if target.list_id != list_id || !target.can_move() {
                return Err(DomainError::InvalidValue(
                    "Reorder tasks within the same list".into(),
                ));
            }
        }
        let mut ids: Vec<_> = self
            .tasks
            .iter()
            .filter(|task| task.list_id == list_id && task.can_move())
            .map(|task| (task.position, task.id))
            .collect();
        ids.sort_unstable();
        let mut ids: Vec<_> = ids
            .into_iter()
            .map(|(_, id)| id)
            .filter(|value| *value != id)
            .collect();
        let position = before
            .and_then(|before_id| ids.iter().position(|value| *value == before_id))
            .unwrap_or(ids.len());
        ids.insert(position, id);
        let mut changes = Changes::default();
        let positions: std::collections::HashMap<_, _> = ids
            .into_iter()
            .enumerate()
            .map(|(position, id)| (id, position as i64))
            .collect();
        for task in &mut self.tasks {
            if let Some(position) = positions.get(&task.id) {
                if task.position == *position {
                    continue;
                }
                task.position = *position;
                task.updated_on = today;
                changes.changed = true;
                changes.changed_tasks.push(task.id);
            }
        }
        Ok(changes)
    }
}
