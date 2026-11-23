use super::{AppViewModel, HostRequest};
use todo_domain::{Action, AppState, Date};

impl AppViewModel {
    pub fn export_json(&self) -> Result<String, String> {
        self.document
            .borrow()
            .to_json()
            .map_err(|error| error.to_string())
    }

    pub fn import_json(&self, json: &str) -> Result<(), String> {
        self.replace_document(json, true, "Backup restored")
    }

    /// The host must perform its export/compare check immediately before this synchronous call.
    pub fn replace_from_cloud(&self, json: &str) -> Result<(), String> {
        self.replace_document(json, false, "Synced with Microsoft")
    }

    /// Pure preflight for the host journal: returns exactly what a cloud replacement will save.
    pub fn prepare_cloud_json(&self, json: &str) -> Result<String, String> {
        self.prepare_document(json, false)?
            .to_json()
            .map_err(|error| error.to_string())
    }

    pub fn prepare_import_json(&self, json: &str) -> Result<String, String> {
        self.prepare_document(json, true)?
            .to_json()
            .map_err(|error| error.to_string())
    }

    fn prepare_document(&self, json: &str, record_undo: bool) -> Result<AppState, String> {
        let mut document = AppState::from_json(json).map_err(|error| error.to_string())?;
        if record_undo {
            // Imported data cannot opt this installation into resident or scheduled OS work.
            // Account workspace activation uses its own replacement path and retains local consent.
            document.settings.keep_running_in_background = false;
            document.settings.background_reminders = false;
        } else {
            // Root navigation order is local-only even when an older Graph adapter omits it.
            document.sidebar_order = self.document.borrow().sidebar_order.clone();
            document.repair_sidebar_order();
        }
        {
            let delivered = self.delivered_reminders.borrow();
            for task in &mut document.tasks {
                if let Some(reminder) = &mut task.reminder {
                    if delivered.contains(&(
                        task.id,
                        reminder.date,
                        reminder.minutes_after_midnight,
                    )) {
                        reminder.notified = true;
                    }
                }
            }
        }
        Ok(document)
    }

    fn replace_document(&self, json: &str, record_undo: bool, message: &str) -> Result<(), String> {
        let document = self.prepare_document(json, record_undo)?;
        self.clear_task_clipboard();
        if *self.document.borrow() == document {
            if self.dirty.get() {
                self.persist()?;
            }
            self.set_status(message.into());
            return Ok(());
        }
        let serialized = document.to_json().map_err(|error| error.to_string())?;
        // Failed imports leave the existing document intact, and never claim a saved backup.
        if let Err(error) = (self.save)(&serialized) {
            let error = format!("Could not save your changes: {error}");
            self.report_error(error.clone());
            return Err(error);
        }
        let mut previous = super::history::HistoryEntry::whole(&self.document.borrow());
        previous.include_new_ids(&document);
        self.document.replace(document);
        self.dirty.set(false);
        if record_undo {
            self.push_undo(previous);
        } else {
            self.undo.borrow_mut().clear();
        }
        self.redo.borrow_mut().clear();
        self.reload_cache();
        self.set_status(message.into());
        Ok(())
    }

    pub(crate) fn persist(&self) -> Result<(), String> {
        let result = self.export_json().and_then(|text| (self.save)(&text));
        match &result {
            Ok(()) => {
                self.dirty.set(false);
                if self
                    .status
                    .borrow()
                    .starts_with("Could not save your changes:")
                {
                    self.status.borrow_mut().clear();
                }
            }
            Err(error) => {
                self.dirty.set(true);
                self.status_undo.set(false);
                *self.status.borrow_mut() = format!("Could not save your changes: {error}")
            }
        }
        result
    }

    pub fn export_backup(&self) {
        match self.export_json() {
            Ok(contents) => self.request(HostRequest::Export {
                filename: format!("todo-backup-{}.json", self.today()),
                contents,
            }),
            Err(error) => self.report_error(error),
        }
    }

    /// Delivery runs only while a host is open. Mark before invoking host callbacks so a
    /// reentrant clock tick cannot send the same reminder twice. One tick is one disk write.
    pub fn tick(&self, today: Date, minutes: u16) {
        self.set_current_minutes(minutes);
        self.set_today(today);
        let pending = self
            .state()
            .reminders_due(today, minutes)
            .into_iter()
            .filter_map(|task| {
                task.reminder
                    .clone()
                    .map(|reminder| (task.id, task.title.clone(), reminder))
            })
            .collect::<Vec<_>>();
        if pending.is_empty() {
            return;
        }
        self.dirty.set(true);
        let mut changed = todo_domain::Changes::default();
        let mut requests = Vec::new();
        for (id, title, mut reminder) in pending {
            let key = (id, reminder.date, reminder.minutes_after_midnight);
            let first = self.delivered_reminders.borrow_mut().insert(key);
            reminder.notified = true;
            let result = self.document.borrow_mut().apply(
                Action::SetReminder {
                    id,
                    reminder: Some(reminder),
                },
                today,
            );
            if let Ok(changes) = result {
                changed.changed |= changes.changed;
                changed.changed_tasks.extend(changes.changed_tasks);
                if first {
                    requests.push(HostRequest::Notify {
                        title,
                        body: "Your task reminder".into(),
                        task_id: id,
                        date: key.1,
                        minutes: key.2,
                    });
                }
            }
        }
        self.publish(&changed);
        let _ = self.persist();
        self.notify();
        for request in requests {
            self.request(request);
        }
    }

    pub fn print_list(&self) {
        let mut html = String::new();
        for task in self.visible_tasks() {
            html.push_str(&format!(
                "<article><h3>{} {}</h3>",
                if task.completed { "☑" } else { "☐" },
                escape(&task.title)
            ));
            if let Some(due) = task.due_date {
                html.push_str(&format!("<p>Due {due}</p>"));
            }
            for step in task.steps {
                html.push_str(&format!(
                    "<p>{} {}</p>",
                    if step.completed { "☑" } else { "☐" },
                    escape(&step.title)
                ));
            }
            if !task.notes.is_empty() {
                let rich = task
                    .notes_html
                    .as_deref()
                    .and_then(|html| todo_domain::notes::RichNote::from_html(html).ok())
                    .filter(|note| note.text() == task.notes);
                if let Some(note) = rich {
                    html.push_str(&note.to_html());
                } else {
                    html.push_str(&format!("<p class=notes>{}</p>", escape(&task.notes)));
                }
            }
            html.push_str("</article>");
        }
        self.request(HostRequest::Print {
            title: self.title(),
            html,
        });
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
