use super::{AppViewModel, HostRequest, Panel};
use todo_domain::{AppState, SortOrder, View};

impl AppViewModel {
    /// Account IDs define separate entity namespaces. Do not reuse import undo,
    /// cloud sidebar preservation, or delivered-reminder IDs across namespaces.
    pub fn replace_account_document(&self, json: &str) -> Result<(), String> {
        if self.applying.get() {
            return Err("Finish the current task operation before switching accounts.".into());
        }
        let document = AppState::from_json(json).map_err(|error| error.to_string())?;
        let serialized = document.to_json().map_err(|error| error.to_string())?;
        (self.save)(&serialized)?;
        self.clear_task_clipboard();
        self.undo.borrow_mut().clear();
        self.redo.borrow_mut().clear();
        self.pending.borrow_mut().clear();
        self.delivered_reminders.borrow_mut().clear();
        self.step_drafts.borrow_mut().clear();
        self.navigation_history.borrow_mut().clear();
        self.rename.borrow_mut().take();
        self.outlook_categories.borrow_mut().clear();
        self.selected.set(None);
        self.selection.borrow_mut().clear();
        self.selection_mode.set(false);
        self.selection_anchor.set(None);
        self.task_cursor.set(None);
        self.refreshing.set(false);
        self.status_undo.set(false);
        *self.view.borrow_mut() = View::MyDay;
        *self.panel.borrow_mut() = Panel::None;
        self.sort.set(SortOrder::Manual);
        self.sort_reversed.set(false);
        let theme = document.settings.theme;
        self.document.replace(document);
        self.dirty.set(false);
        self.reload_cache();
        self.request(HostRequest::SetTheme { theme });
        self.set_status("Account workspace opened".into());
        Ok(())
    }
}
