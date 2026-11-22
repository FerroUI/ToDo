use super::{history::HistoryEntry, AppViewModel, HostRequest, Panel};
use ferroui_reactive::ObservableExt;
use todo_domain::{Action, Changes, Id, SortOrder, View, TASKS_LIST};

type Applied = Box<dyn FnOnce(&AppViewModel, &Changes)>;

pub(crate) struct PendingAction {
    action: Action,
    applied: Option<Applied>,
}

impl PendingAction {
    pub fn new(action: Action) -> Self {
        Self {
            action,
            applied: None,
        }
    }
}

impl AppViewModel {
    /// Completion follows this exact queued command, including when a cache callback reenters.
    pub(crate) fn dispatch_after(
        &self,
        action: Action,
        applied: impl FnOnce(&Self, &Changes) + 'static,
    ) {
        self.pending.borrow_mut().push_back(PendingAction {
            action,
            applied: Some(Box::new(applied)),
        });
        if !self.applying.get() {
            let _subscription = self.execute.execute(None).subscribe_next(|_| {});
        }
    }

    pub(crate) fn apply(&self, action: Option<Action>) {
        if let Some(action) = action {
            self.pending
                .borrow_mut()
                .push_back(PendingAction::new(action));
        }
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

    fn apply_one(&self, command: PendingAction) {
        let PendingAction { action, applied } = command;
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
                if let Some(applied) = applied {
                    applied(self, &changes);
                }
                let _ = self.persist();
                self.notify();
            }
            Ok(changes) => {
                if let Some(applied) = applied {
                    applied(self, &changes);
                }
            }
            Err(error) => self.report_error(error.to_string()),
        }
    }

    pub(crate) fn publish(&self, changes: &Changes) {
        let updated =
            {
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
                let obsolete_rename = self.rename.borrow().as_ref().is_some_and(|edit| match edit
                    .target
                {
                    crate::RenameTarget::List(id) => document.list(id).is_none(),
                    crate::RenameTarget::Group(id) => document.group(id).is_none(),
                });
                if obsolete_rename {
                    self.rename.borrow_mut().take();
                }
                if let View::List(id) = self.view() {
                    if let Some(list) = document.list(id) {
                        self.sort.set(list.sort);
                        self.sort_reversed.set(list.sort_reversed);
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
            self.set_undo_status("Task deleted. You can undo this change.".into());
        }
    }

    pub fn set_sort(&self, sort: SortOrder) {
        self.sort.set(sort);
        self.sort_reversed.set(false);
        if let View::List(id) = self.view() {
            self.dispatch(Action::SetListSort { id, sort });
        }
        self.show_panel(Panel::None);
    }

    pub fn sort(&self) -> SortOrder {
        self.sort.get()
    }

    pub fn sort_reversed(&self) -> bool {
        self.sort.get() != SortOrder::Manual && self.sort_reversed.get()
    }

    pub fn reverse_sort(&self) {
        if self.sort() == SortOrder::Manual {
            return;
        }
        let reversed = !self.sort_reversed();
        self.sort_reversed.set(reversed);
        if let View::List(id) = self.view() {
            self.dispatch(Action::SetListSortDirection { id, reversed });
        } else {
            self.notify();
        }
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
