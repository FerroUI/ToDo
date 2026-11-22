use super::{AppViewModel, Panel};
use std::collections::HashSet;
use todo_domain::{Action, Id, View};

impl AppViewModel {
    pub fn selection_mode(&self) -> bool {
        self.selection_mode.get()
    }
    pub fn selection_count(&self) -> usize {
        self.selection.borrow().len()
    }
    pub fn task_cursor(&self) -> Option<Id> {
        self.task_cursor
            .get()
            .filter(|id| self.state().task(*id).is_some())
    }
    pub fn set_task_cursor(&self, id: Option<Id>) {
        if id.is_some_and(|id| self.state().task(id).is_none()) {
            return;
        }
        if self.task_cursor.replace(id) != id {
            self.notify();
        }
    }
    pub fn begin_selection(&self) {
        self.task_cursor.set(self.task_cursor().or(self.selected()));
        self.selected.set(None);
        self.selection_mode.set(true);
        *self.panel.borrow_mut() = Panel::None;
        self.notify();
    }
    pub fn end_selection(&self) {
        self.clear_selection_state();
        self.notify();
    }
    pub fn toggle_task_selection(&self, id: Id) {
        if self.state().task(id).is_none() {
            return;
        }
        self.selection_mode.set(true);
        self.selected.set(None);
        self.task_cursor.set(Some(id));
        self.selection_anchor.set(Some(id));
        self.toggle_selection(id);
    }
    pub fn select_all_tasks(&self, ids: &[Id]) {
        let existing = self.existing_task_ids();
        *self.selection.borrow_mut() = ids
            .iter()
            .copied()
            .filter(|id| existing.contains(id))
            .collect();
        self.selection_mode.set(true);
        self.selected.set(None);
        self.selection_anchor.set(ids.first().copied());
        self.task_cursor
            .set(self.task_cursor().or(ids.first().copied()));
        self.notify();
    }
    pub fn select_task_range(&self, ids: &[Id], target: Id) {
        let existing = self.existing_task_ids();
        let ids: Vec<_> = ids
            .iter()
            .copied()
            .filter(|id| existing.contains(id))
            .collect();
        let Some(end) = ids.iter().position(|id| *id == target) else {
            return;
        };
        let anchor = self
            .selection_anchor
            .get()
            .or(self.task_cursor())
            .or(self.selected());
        let start = anchor
            .and_then(|id| ids.iter().position(|candidate| *candidate == id))
            .unwrap_or(end);
        self.selection_anchor.set(Some(ids[start]));
        *self.selection.borrow_mut() = ids[start.min(end)..=start.max(end)]
            .iter()
            .copied()
            .collect();
        self.selection_mode.set(true);
        self.selected.set(None);
        self.task_cursor.set(Some(target));
        self.notify();
    }

    /// Matches the displayed date groups and completed-group visibility.
    pub fn visible_task_ids(&self) -> Vec<Id> {
        let view = self.view();
        let state = self.state();
        let show = match view {
            View::List(id) => state.list(id).is_some_and(|list| list.show_completed),
            _ => state.settings.show_completed,
        };
        let today = self.today();
        let mut tasks = state.visible_tasks_with_direction(
            &view,
            self.sort(),
            self.sort_reversed(),
            show,
            today,
        );
        if view == View::Planned {
            tasks.sort_by_key(|task| {
                let date = task
                    .due_date
                    .or_else(|| task.reminder.as_ref().map(|r| r.date));
                let group = match date {
                    Some(date) if date < today => 0,
                    Some(date) if date == today => 1,
                    Some(date) if Some(date) == today.succ_opt() => 2,
                    Some(_) => 3,
                    None => 4,
                };
                (task.completed, group)
            });
        }
        tasks
            .into_iter()
            .filter(|task| !task.completed || view == View::Completed || self.completed_expanded())
            .map(|task| task.id)
            .collect()
    }

    pub fn complete_selected_tasks(&self) {
        let ids = self.existing_selection();
        if ids.is_empty() {
            return;
        }
        self.clear_selection_state();
        self.dispatch(Action::Batch(
            ids.into_iter()
                .map(|id| Action::SetCompleted {
                    id,
                    completed: true,
                })
                .collect(),
        ));
        self.notify();
    }
    pub fn delete_selection(&self) {
        let ids = self.existing_selection();
        if ids.is_empty()
            || !ids
                .iter()
                .all(|id| self.state().task(*id).is_some_and(|task| task.can_delete()))
        {
            return;
        }
        let confirm = self.state().settings.confirm_delete;
        if confirm {
            self.show_panel(Panel::DeleteSelection);
        } else {
            self.confirm_delete_selection();
        }
    }
    pub fn confirm_delete_selection(&self) {
        let ids = self.existing_selection();
        if ids.is_empty()
            || !ids
                .iter()
                .all(|id| self.state().task(*id).is_some_and(|task| task.can_delete()))
        {
            return;
        }
        let count = ids.len();
        self.dispatch_after(
            Action::Batch(ids.iter().copied().map(Action::DeleteTask).collect()),
            move |vm, changes| {
                if ids.iter().all(|id| changes.deleted_tasks.contains(id)) {
                    if vm.selection().iter().all(|id| ids.contains(id)) {
                        vm.clear_selection_state();
                    }
                    if vm.panel() == Panel::DeleteSelection {
                        *vm.panel.borrow_mut() = Panel::None;
                    }
                    vm.set_undo_status(format!("{count} tasks deleted. You can undo this change."));
                }
            },
        );
    }
    pub fn move_selection(&self, list_id: Id) {
        let ids = self.existing_selection();
        if ids.is_empty()
            || self.state().list(list_id).is_none()
            || !ids
                .iter()
                .all(|id| self.state().task(*id).is_some_and(|task| task.can_move()))
        {
            return;
        }
        let count = ids.len();
        self.dispatch_after(
            Action::Batch(
                ids.iter()
                    .copied()
                    .map(|id| Action::MoveTask { id, list_id })
                    .collect(),
            ),
            move |vm, changes| {
                if ids.iter().all(|id| {
                    vm.state()
                        .task(*id)
                        .is_some_and(|task| task.list_id == list_id)
                }) {
                    if vm.selection().iter().all(|id| ids.contains(id)) {
                        vm.clear_selection_state();
                    }
                    if vm.panel() == Panel::MoveSelection {
                        *vm.panel.borrow_mut() = Panel::None;
                    }
                    if changes.changed {
                        vm.set_undo_status(format!(
                            "{count} tasks moved. You can undo this change."
                        ));
                    } else {
                        vm.notify();
                    }
                }
            },
        );
    }
    fn clear_selection_state(&self) {
        self.selection.borrow_mut().clear();
        self.selection_mode.set(false);
        self.selection_anchor.set(None);
    }
    fn existing_task_ids(&self) -> HashSet<Id> {
        self.state().tasks.iter().map(|task| task.id).collect()
    }
    fn existing_selection(&self) -> Vec<Id> {
        let selected = self.selection.borrow();
        self.state()
            .tasks
            .iter()
            .filter(|task| selected.contains(&task.id))
            .map(|task| task.id)
            .collect()
    }
}
