use crate::{AppViewModel, Panel};
use todo_domain::{Action, Id, View, TASKS_LIST};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenameTarget {
    List(Id),
    Group(Id),
}

/// An editor transaction, deliberately excluded from persistence and undo history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameSession {
    pub target: RenameTarget,
    pub draft: String,
}

impl AppViewModel {
    pub fn rename_session(&self) -> Option<RenameSession> {
        self.rename.borrow().clone()
    }

    pub fn begin_rename(&self, target: RenameTarget) {
        let name = {
            let state = self.state();
            match target {
                RenameTarget::List(id) if id != TASKS_LIST => {
                    state.list(id).map(|list| list.name.clone())
                }
                RenameTarget::Group(id) => state.group(id).map(|group| group.name.clone()),
                _ => None,
            }
        };
        let Some(draft) = name else { return };
        if let RenameTarget::List(id) = target {
            if self.view() != View::List(id) || !self.layout().list_visible {
                self.navigate(View::List(id));
            }
        }
        *self.panel.borrow_mut() = Panel::None;
        if matches!(target, RenameTarget::Group(_)) {
            if self.layout().can_inline_navigation {
                self.sidebar.set(true);
            } else {
                self.navigation_open.set(true);
            }
        }
        *self.rename.borrow_mut() = Some(RenameSession { target, draft });
        self.notify();
    }

    pub fn set_rename_draft(&self, target: RenameTarget, draft: String) {
        if let Some(edit) = self
            .rename
            .borrow_mut()
            .as_mut()
            .filter(|edit| edit.target == target)
        {
            edit.draft = draft;
        }
    }

    /// Clear the transaction before dispatch: removing a focused editor can raise LostFocus.
    pub fn commit_rename(&self, target: RenameTarget) {
        let edit = {
            let mut current = self.rename.borrow_mut();
            if current.as_ref().is_none_or(|edit| edit.target != target) {
                return;
            }
            current.take().unwrap()
        };
        let name = edit.draft.trim().to_owned();
        if !name.is_empty() {
            let action = match target {
                RenameTarget::List(id) => self
                    .state()
                    .list(id)
                    .filter(|list| list.name != name)
                    .map(|_| Action::RenameList { id, name }),
                RenameTarget::Group(id) => self
                    .state()
                    .group(id)
                    .filter(|group| group.name != name)
                    .map(|_| Action::RenameGroup { id, name }),
            };
            if let Some(action) = action {
                self.dispatch(action);
            }
        }
        self.notify();
    }

    pub fn cancel_rename(&self, target: RenameTarget) {
        let matches = self
            .rename
            .borrow()
            .as_ref()
            .is_some_and(|edit| edit.target == target);
        if matches {
            self.rename.borrow_mut().take();
            self.notify();
        }
    }

    pub fn create_list_inline(&self) {
        let id = self.state().next_id;
        self.dispatch(Action::AddList {
            name: "Untitled list".into(),
        });
        if self.state().list(id).is_some() {
            self.begin_rename(RenameTarget::List(id));
        }
    }

    pub fn create_group_inline(&self) {
        let id = self.state().next_id;
        self.dispatch(Action::AddGroup {
            name: "Untitled group".into(),
        });
        if self.state().group(id).is_some() {
            self.begin_rename(RenameTarget::Group(id));
        }
    }
}
