use super::{view_title, AppViewModel};
use todo_domain::{Action, Id, SortOrder, View, TASKS_LIST};

impl AppViewModel {
    /// Reorder the named list among its siblings without changing the active view.
    pub fn move_list_by(&self, id: Id, delta: i32) {
        if id == TASKS_LIST || delta == 0 {
            return;
        }
        let before = {
            let state = self.state();
            let Some(list) = state.list(id) else {
                return;
            };
            let siblings = state
                .lists
                .iter()
                .filter(|candidate| {
                    candidate.id != TASKS_LIST && candidate.group_id == list.group_id
                })
                .map(|candidate| (candidate.position, candidate.id))
                .collect();
            before_at_delta(siblings, id, delta)
        };
        if let Some(before) = before {
            self.dispatch(Action::ReorderList { id, before });
        }
    }

    /// Groups are siblings of other groups; this does not reorder root lists.
    pub fn move_group_by(&self, id: Id, delta: i32) {
        if delta == 0 {
            return;
        }
        let before = {
            let state = self.state();
            let siblings = state
                .groups
                .iter()
                .map(|group| (group.position, group.id))
                .collect();
            before_at_delta(siblings, id, delta)
        };
        if let Some(before) = before {
            self.dispatch(Action::ReorderGroup { id, before });
        }
    }

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

fn before_at_delta(mut siblings: Vec<(i64, Id)>, id: Id, delta: i32) -> Option<Option<Id>> {
    siblings.sort_unstable();
    let mut ids: Vec<_> = siblings.into_iter().map(|(_, id)| id).collect();
    let index = ids.iter().position(|candidate| *candidate == id)?;
    let destination = index
        .saturating_add_signed(delta as isize)
        .min(ids.len() - 1);
    if destination == index {
        return None;
    }
    ids.remove(index);
    Some(ids.get(destination).copied())
}
