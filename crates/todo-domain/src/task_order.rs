use crate::{Action, AppState, Changes, Date, Id};
use std::collections::HashMap;

impl AppState {
    /// Imported order keys can use the full i64 range. Repair only an append's
    /// destination and only when it has no free successor key.
    pub(crate) fn task_order_needs_repair(&self, action: &Action, today: Date) -> Option<Id> {
        let list_id = match action {
            Action::AddTask { list_id, .. } => Some(*list_id),
            Action::AddTaskInView { view, .. } | Action::AddTaskEntry { view, .. } => {
                Some(view.destination_list())
            }
            Action::DuplicateTask(id) | Action::PromoteStep { task_id: id, .. } => {
                self.task(*id).map(|task| task.list_id)
            }
            Action::MoveTask { id, list_id } => self
                .task(*id)
                .filter(|task| task.list_id != *list_id)
                .map(|_| *list_id),
            Action::ToggleCompleted(id)
            | Action::SetCompleted {
                id,
                completed: true,
            } => self
                .task(*id)
                .filter(|task| !task.completed && !task.recurrence_generated)
                .filter(|task| {
                    task.recurrence.as_ref().is_some_and(|repeat| {
                        repeat
                            .next_after(task.due_date.unwrap_or(today), today)
                            .is_some()
                    })
                })
                .map(|task| task.list_id),
            _ => None,
        }?;
        self.tasks
            .iter()
            .any(|task| task.list_id == list_id && task.can_move() && task.position == i64::MAX)
            .then_some(list_id)
    }

    pub(crate) fn compact_task_order(&mut self, list_id: Id, today: Date) -> Changes {
        let mut order: Vec<_> = self
            .tasks
            .iter()
            .filter(|task| task.list_id == list_id && task.can_move())
            .map(|task| (task.position, task.id))
            .collect();
        order.sort_unstable();
        let positions: HashMap<_, _> = order
            .into_iter()
            .enumerate()
            .map(|(position, (_, id))| (id, position as i64))
            .collect();
        let mut changes = Changes::default();
        for task in &mut self.tasks {
            if let Some(position) = positions.get(&task.id) {
                if task.position != *position {
                    task.position = *position;
                    task.updated_on = today;
                    changes.changed_tasks.push(task.id);
                    changes.changed = true;
                }
            }
        }
        changes
    }
}
