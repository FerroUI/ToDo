use super::{history::HistoryEntry, AppViewModel};
use ferroui_dynamic_data::IObservableCache;
use std::collections::HashSet;

impl AppViewModel {
    pub fn can_undo(&self) -> bool {
        !self.undo.borrow().is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.borrow().is_empty()
    }

    pub(crate) fn push_undo(&self, entry: HistoryEntry) {
        self.status_undo.set(false);
        let mut undo = self.undo.borrow_mut();
        undo.push_back(entry);
        let mut bytes: usize = undo.iter().map(HistoryEntry::estimated_bytes).sum();
        while undo.len() > 1 && (undo.len() > 30 || bytes > 8 * 1024 * 1024) {
            if let Some(oldest) = undo.pop_front() {
                bytes = bytes.saturating_sub(oldest.estimated_bytes());
            }
        }
    }

    pub fn undo(&self) {
        let previous = self.undo.borrow_mut().pop_back();
        if let Some(previous) = previous {
            self.status_undo.set(false);
            self.dirty.set(true);
            let (inverse, changes) = previous.restore(&mut self.document.borrow_mut());
            self.redo.borrow_mut().push(inverse);
            self.preserve_deliveries();
            self.publish(&changes);
            if self.persist().is_ok() {
                self.set_status("Change undone".into());
            } else {
                self.notify();
            }
        }
    }

    pub fn redo(&self) {
        let next = self.redo.borrow_mut().pop();
        if let Some(next) = next {
            self.dirty.set(true);
            let (inverse, changes) = next.restore(&mut self.document.borrow_mut());
            self.push_undo(inverse);
            self.preserve_deliveries();
            self.publish(&changes);
            if self.persist().is_ok() {
                self.set_status("Change restored".into());
            } else {
                self.notify();
            }
        }
    }

    pub(crate) fn preserve_deliveries(&self) {
        let delivered = self.delivered_reminders.borrow();
        for task in &mut self.document.borrow_mut().tasks {
            if let Some(reminder) = &mut task.reminder {
                if delivered.contains(&(task.id, reminder.date, reminder.minutes_after_midnight)) {
                    reminder.notified = true;
                }
            }
        }
    }

    pub(crate) fn reload_cache(&self) {
        let snapshot = self.document.borrow().tasks.clone();
        let ids: HashSet<_> = snapshot.iter().map(|task| task.id).collect();
        let removed: Vec<_> = self
            .tasks
            .keys()
            .into_iter()
            .filter(|id| !ids.contains(id))
            .collect();
        let updated: Vec<_> = snapshot
            .into_iter()
            .filter(|task| self.tasks.lookup(&task.id).as_ref() != Some(task))
            .collect();
        self.tasks.edit(|cache| {
            for id in removed {
                cache.remove_key(&id);
            }
            for task in updated {
                cache.add_or_update(task);
            }
        });
        // Reconcile navigation and selection too when imports or undo remove their targets.
        self.publish(&todo_domain::Changes::default());
    }
}
