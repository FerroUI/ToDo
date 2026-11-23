//! Ordered bulk transfers validate the whole operation before changing any record.
use crate::{AppState, Changes, Date, DomainError, Id, Task, View};
use std::collections::{HashMap, HashSet};

impl AppState {
    pub(crate) fn transfer_tasks(
        &mut self,
        ids: &[Id],
        list_id: Id,
        before: Option<Id>,
        today: Date,
    ) -> Result<Changes, DomainError> {
        self.list_index(list_id)?;
        if ids.is_empty() {
            return Ok(Changes::default());
        }
        let selected: HashSet<_> = ids.iter().copied().collect();
        if selected.len() != ids.len() {
            return Err(DomainError::InvalidValue(
                "Select each task only once".into(),
            ));
        }
        let tasks: HashMap<_, _> = self.tasks.iter().map(|task| (task.id, task)).collect();
        for id in ids {
            let task = tasks.get(id).ok_or(DomainError::NotFound("Task", *id))?;
            if !task.can_move() {
                return Err(DomainError::InvalidValue(
                    "This task stays in its source list. Duplicate it to create an independent task you can move.".into(),
                ));
            }
        }
        if let Some(id) = before {
            let target = tasks.get(&id).ok_or(DomainError::NotFound("Task", id))?;
            if selected.contains(&id) || target.list_id != list_id || !target.can_move() {
                return Err(DomainError::InvalidValue(
                    "Choose a task outside the selection in the destination list".into(),
                ));
            }
        }
        let mut previous: Vec<_> = self
            .tasks
            .iter()
            .filter(|task| task.list_id == list_id && task.can_move())
            .map(|task| (task.position, task.id))
            .collect();
        previous.sort_unstable();
        let previous: Vec<_> = previous.into_iter().map(|(_, id)| id).collect();
        let mut order: Vec<_> = previous
            .iter()
            .copied()
            .filter(|id| !selected.contains(id))
            .collect();
        let index = before
            .and_then(|id| order.iter().position(|candidate| *candidate == id))
            .unwrap_or(order.len());
        order.splice(index..index, ids.iter().copied());
        if order == previous {
            return Ok(Changes::default());
        }
        let positions: HashMap<_, _> = order
            .into_iter()
            .enumerate()
            .map(|(position, id)| (id, position as i64))
            .collect();
        let mut changes = Changes::default();
        for task in &mut self.tasks {
            if let Some(position) = positions.get(&task.id) {
                if task.list_id != list_id || task.position != *position {
                    task.list_id = list_id;
                    task.position = *position;
                    task.updated_on = today;
                    changes.changed = true;
                    changes.changed_tasks.push(task.id);
                }
            }
        }
        Ok(changes)
    }

    pub(crate) fn paste_tasks(
        &mut self,
        tasks: Vec<Task>,
        view: View,
        today: Date,
    ) -> Result<Changes, DomainError> {
        let list_id = view.destination_list();
        self.list_index(list_id)?;
        if tasks.is_empty() {
            return Ok(Changes::default());
        }
        let required = tasks.iter().try_fold(0_u64, |total, task| {
            total
                .checked_add(1)?
                .checked_add(task.steps.len() as u64)?
                .checked_add(task.attachments.len() as u64)
        });
        if required
            .and_then(|count| self.next_id.checked_add(count))
            .is_none_or(|next| next >= i64::MAX as u64)
        {
            return Err(DomainError::InvalidValue(
                "There are no available task identifiers".into(),
            ));
        }
        // External clipboard contents are never permitted to leave a partial import.
        let mut transaction = self.clone();
        let mut changes = Changes::default();
        let last = transaction
            .tasks
            .iter()
            .filter(|task| task.list_id == list_id && task.can_move())
            .map(|task| task.position)
            .max()
            .unwrap_or(-1);
        if last.checked_add(tasks.len() as i64).is_none() {
            changes.merge(transaction.compact_task_order(list_id, today));
        }
        for original in tasks {
            let position = transaction.next_task_position(list_id)?;
            let mut task = transaction.duplicate(original, today, position);
            task.title = crate::action::nonempty(task.title)?;
            task.list_id = list_id;
            // Provider snapshots/remote IDs must not become a second synchronization identity.
            task.microsoft_data = None;
            for resource in &mut task.linked_resources {
                resource.id.clear();
                resource.external_id.clear();
            }
            if let Some(reminder) = &mut task.reminder {
                reminder.notified = false;
            }
            match view {
                View::MyDay => {
                    task.my_day = Some(today);
                    task.my_day_excluded = None;
                }
                View::Important => task.important = true,
                View::Planned if task.due_date.is_none() && task.reminder.is_none() => {
                    task.due_date = Some(task.start_date.unwrap_or(today).max(today));
                }
                View::AssignedToMe => task.assigned_to = Some("me".into()),
                _ => {}
            }
            if let Some(rule) = &mut task.recurrence {
                rule.restart_from(task.due_date.unwrap_or(today));
            }
            changes.created_task = Some(task.id);
            changes.changed_tasks.push(task.id);
            changes.changed = true;
            transaction.tasks.push(task);
        }
        transaction.validate()?;
        *self = transaction;
        Ok(changes)
    }
}
