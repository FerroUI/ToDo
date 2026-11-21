use std::collections::HashMap;
use todo_domain::{AppState, Date, Id, View};
/// All sidebar counters are computed in one pass, independent of the number of custom lists.
pub(super) struct Counts {
    lists: HashMap<Id, usize>,
    smart: [usize; 7],
}
impl Counts {
    pub(super) fn new(state: &AppState, today: Date) -> Self {
        let mut result = Self {
            lists: HashMap::new(),
            smart: [0; 7],
        };
        for task in &state.tasks {
            if task.completed {
                result.smart[6] += 1;
                continue;
            }
            *result.lists.entry(task.list_id).or_default() += 1;
            result.smart[0] += usize::from(task.is_my_day(today));
            result.smart[1] += usize::from(task.important);
            result.smart[2] += usize::from(task.due_date.is_some() || task.reminder.is_some());
            result.smart[3] += usize::from(task.assigned_to.as_deref() == Some("me"));
            result.smart[4] += usize::from(task.flagged_email);
            result.smart[5] += 1;
        }
        result
    }
    pub(super) fn get(&self, view: &View) -> usize {
        match view {
            View::List(id) => self.lists.get(id).copied().unwrap_or_default(),
            View::MyDay => self.smart[0],
            View::Important => self.smart[1],
            View::Planned => self.smart[2],
            View::AssignedToMe => self.smart[3],
            View::FlaggedEmail => self.smart[4],
            View::All => self.smart[5],
            View::Completed => self.smart[6],
            View::Search(_) => 0,
        }
    }
}
