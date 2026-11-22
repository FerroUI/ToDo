use super::{view_title, AppViewModel};
use todo_domain::{Date, Task, View};

impl AppViewModel {
    pub fn title(&self) -> String {
        match self.view() {
            View::List(id) => self
                .state()
                .list(id)
                .map(|list| list.name.clone())
                .unwrap_or_else(|| "Tasks".into()),
            View::Search(text) => {
                if text.is_empty() {
                    "Search".into()
                } else {
                    format!("Results for \"{text}\"")
                }
            }
            view => view_title(&view).into(),
        }
    }

    pub fn visible_tasks(&self) -> Vec<Task> {
        let state = self.state();
        let show = match self.view() {
            View::List(id) => state.list(id).is_some_and(|list| list.show_completed),
            _ => state.settings.show_completed,
        };
        state
            .visible_tasks_with_direction(
                &self.view(),
                self.sort(),
                self.sort_reversed(),
                show,
                self.today(),
            )
            .into_iter()
            .cloned()
            .collect()
    }

    pub fn count(&self, view: &View) -> usize {
        let count = self.state().count(view, self.today());
        if *view == View::Completed {
            count.completed
        } else {
            count.active
        }
    }

    pub fn suggestions(&self) -> Vec<Task> {
        let state = self.state();
        state
            .suggestions(self.today())
            .into_iter()
            .take(40)
            .cloned()
            .collect()
    }

    pub fn accent(&self) -> String {
        let view = self.view();
        if !matches!(view, View::List(_)) {
            if let Some(color) = self.state().settings.smart_colors.get(view_title(&view)) {
                return color.clone();
            }
        }
        match self.view() {
            View::Important => "#C34F72".into(),
            View::Planned => "#228D76".into(),
            View::All => "#287C9C".into(),
            View::Completed => "#588568".into(),
            View::MyDay => "#58699D".into(),
            View::List(id) => self
                .state()
                .list(id)
                .map(|l| l.color.clone())
                .unwrap_or_else(|| "#3E69E4".into()),
            _ => "#3E69E4".into(),
        }
    }

    pub fn subtitle(&self) -> String {
        if matches!(self.view(), View::MyDay) {
            self.today().format("%A, %B %-d").to_string()
        } else {
            let count = if matches!(self.view(), View::Search(_)) {
                let state = self.state();
                let count = state.count(&self.view(), self.today());
                count.active
                    + if state.settings.show_completed {
                        count.completed
                    } else {
                        0
                    }
            } else {
                self.count(&self.view())
            };
            format!("{count} {}", if count == 1 { "task" } else { "tasks" })
        }
    }

    pub fn task_metadata(&self, task: &Task) -> String {
        let mut parts = Vec::new();
        if task.is_planner() {
            parts.push("Planner".into());
        } else if task.flagged_email {
            parts.push("Outlook".into());
        } else if !matches!(self.view(), View::List(_)) {
            if let Some(list) = self.state().list(task.list_id) {
                parts.push(list.name.clone());
            }
        }
        if !task.steps.is_empty() {
            parts.push(format!(
                "{} of {}",
                task.completed_steps(),
                task.steps.len()
            ));
        }
        if let Some(date) = task.due_date {
            parts.push(self.date_label(date));
        }
        if task.is_my_day(self.today()) && !matches!(self.view(), View::MyDay) {
            parts.push("My Day".into());
        }
        if task.recurrence.is_some() {
            parts.push("Repeats".into());
        }
        if !task.attachments.is_empty() {
            parts.push(format!("{} files", task.attachments.len()));
        }
        if !task.notes.is_empty() {
            parts.push("Note".into());
        }
        parts.join("  ·  ")
    }

    pub fn date_label(&self, date: Date) -> String {
        if date == self.today() {
            "Today".into()
        } else if Some(date) == self.today().succ_opt() {
            "Tomorrow".into()
        } else {
            date.format("%a, %-d %b").to_string()
        }
    }
}
