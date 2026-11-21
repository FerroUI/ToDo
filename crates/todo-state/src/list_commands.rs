use super::{view_title, AppViewModel};
use todo_domain::{Action, Id, SortOrder, View};

impl AppViewModel {
    pub fn set_smart_color(&self, color: String) {
        if color.len() != 7
            || !color.starts_with('#')
            || !color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            self.report_error("Choose a six-digit hexadecimal theme color".into());
            return;
        }
        if let View::List(id) = self.view() {
            let background = self
                .state()
                .list(id)
                .and_then(|list| list.background.clone());
            self.dispatch(Action::SetListTheme {
                id,
                color,
                background,
            });
            return;
        }
        let mut settings = self.state().settings.clone();
        settings
            .smart_colors
            .insert(view_title(&self.view()).into(), color);
        self.dispatch(Action::SetSettings(settings));
    }

    pub fn move_task_by(&self, id: Id, delta: i32) {
        if delta == 0 {
            return;
        }
        let (list_id, before) = {
            let state = self.state();
            let Some(task) = state.task(id) else {
                return;
            };
            let mut ids: Vec<_> = state
                .visible_tasks(
                    &View::List(task.list_id),
                    SortOrder::Manual,
                    true,
                    self.today(),
                )
                .into_iter()
                .filter(|candidate| candidate.completed == task.completed)
                .map(|task| task.id)
                .collect();
            let Some(index) = ids.iter().position(|candidate| *candidate == id) else {
                return;
            };
            let destination = index
                .saturating_add_signed(delta as isize)
                .min(ids.len() - 1);
            if destination == index {
                return;
            }
            ids.remove(index);
            (task.list_id, ids.get(destination).copied())
        };
        self.sort.set(SortOrder::Manual);
        self.dispatch(Action::Batch(vec![
            Action::ReorderTask { id, before },
            Action::SetListSort {
                id: list_id,
                sort: SortOrder::Manual,
            },
        ]));
    }
}
