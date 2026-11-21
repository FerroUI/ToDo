use super::{history::HistoryEntry, AppViewModel, HostRequest, Panel};
use todo_domain::{Action, Changes, Id, SortOrder, View, TASKS_LIST};

impl AppViewModel {
    pub(crate) fn apply(&self, action: Action) {
        self.pending.borrow_mut().push_back(action);
        if self.applying.replace(true) {
            return;
        }
        loop {
            let action = self.pending.borrow_mut().pop_front();
            let Some(action) = action else {
                break;
            };
            self.apply_one(action);
        }
        self.applying.set(false);
    }

    fn apply_one(&self, action: Action) {
        let (mut before, completing) = {
            let state = self.document.borrow();
            (
                HistoryEntry::capture(&state, &action),
                completion_ids(&action, &state),
            )
        };
        let result = self.document.borrow_mut().apply(action, self.today());
        match result {
            Ok(changes) if changes.changed => {
                self.dirty.set(true);
                before.finish(&changes);
                self.push_undo(before);
                self.redo.borrow_mut().clear();
                let sound = !completing.is_empty() && self.state().settings.completion_sound;
                self.publish(&changes);
                if sound {
                    self.request(HostRequest::CompletionSound);
                }
                if let Some(id) = changes.created_list {
                    self.navigate(View::List(id));
                }
                let _ = self.persist();
                self.notify();
            }
            Ok(_) => {}
            Err(error) => self.report_error(error.to_string()),
        }
    }

    pub(crate) fn publish(&self, changes: &Changes) {
        let updated = {
            let document = self.document.borrow();
            if self
                .selected
                .get()
                .is_some_and(|id| document.task(id).is_none())
            {
                self.selected.set(None);
            }
            if matches!(self.view(), View::List(id) if document.list(id).is_none()) {
                *self.view.borrow_mut() = View::List(TASKS_LIST);
            }
            self.selection
                .borrow_mut()
                .retain(|id| document.task(*id).is_some());
            if let View::List(id) = self.view() {
                if let Some(list) = document.list(id) {
                    self.sort.set(list.sort);
                }
            }
            changes
                .changed_tasks
                .iter()
                .filter_map(|id| document.task(*id).cloned())
                .collect::<Vec<_>>()
        };
        // Cache notifications and their callbacks must never run under a document borrow.
        self.tasks.edit(|cache| {
            for id in &changes.deleted_tasks {
                cache.remove_key(id);
            }
            for task in updated {
                cache.add_or_update(task);
            }
        });
    }

    pub fn add_task(&self, title: String) {
        let title = title.trim().to_string();
        if title.is_empty() {
            return;
        }
        self.dispatch(Action::AddTaskInView {
            title,
            view: self.view(),
        });
    }

    pub fn delete_task(&self, id: Id) {
        let confirm = self.state().settings.confirm_delete;
        if confirm {
            self.show_panel(Panel::DeleteTask(id));
        } else {
            self.confirm_delete_task(id);
        }
    }

    pub fn confirm_delete_task(&self, id: Id) {
        if self.state().task(id).is_none() {
            return;
        }
        self.dispatch(Action::DeleteTask(id));
        self.show_panel(Panel::None);
        if !self.status().starts_with("Could not save") {
            self.set_status("Task deleted. You can undo this change.".into());
        }
    }

    pub fn set_sort(&self, sort: SortOrder) {
        self.sort.set(sort);
        if let View::List(id) = self.view() {
            self.dispatch(Action::SetListSort { id, sort });
        }
        self.show_panel(Panel::None);
    }

    pub fn sort(&self) -> SortOrder {
        self.sort.get()
    }

    pub fn complete_selection(&self) {
        let actions = self
            .selection()
            .into_iter()
            .map(|id| Action::SetCompleted {
                id,
                completed: true,
            })
            .collect();
        self.dispatch(Action::Batch(actions));
        self.selection.borrow_mut().clear();
        self.notify();
    }
}

fn completion_ids(action: &Action, state: &todo_domain::AppState) -> Vec<Id> {
    match action {
        Action::ToggleCompleted(id)
        | Action::SetCompleted {
            id,
            completed: true,
        } if state.task(*id).is_some_and(|task| !task.completed) => vec![*id],
        Action::Batch(actions) => actions
            .iter()
            .flat_map(|action| completion_ids(action, state))
            .collect(),
        _ => Vec::new(),
    }
}
