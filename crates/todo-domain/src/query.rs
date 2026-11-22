use crate::{AppState, Date, Id, TASKS_LIST, Task};
use chrono::{Datelike, Duration};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    #[default]
    MyDay,
    Important,
    Planned,
    All,
    Completed,
    AssignedToMe,
    FlaggedEmail,
    List(Id),
    Search(String),
}

impl View {
    pub fn title(&self, state: &AppState) -> String {
        match self {
            Self::MyDay => "My Day",
            Self::Important => "Important",
            Self::Planned => "Planned",
            Self::All => "All",
            Self::Completed => "Completed",
            Self::AssignedToMe => "Assigned to me",
            Self::FlaggedEmail => "Flagged email",
            Self::List(id) => {
                return state
                    .list(*id)
                    .map(|list| list.name.clone())
                    .unwrap_or_else(|| "Tasks".into());
            }
            Self::Search(query) => return format!("Results for \"{query}\""),
        }
        .into()
    }

    pub fn destination_list(&self) -> Id {
        if let Self::List(id) = self {
            *id
        } else {
            TASKS_LIST
        }
    }

    pub fn matches(&self, task: &Task, today: Date) -> bool {
        match self {
            Self::MyDay => task.is_my_day(today),
            Self::Important => task.important,
            Self::Planned => task.due_date.is_some() || task.reminder.is_some(),
            Self::All => true,
            Self::Completed => task.completed,
            Self::AssignedToMe => task.assigned_to.as_deref() == Some("me"),
            Self::FlaggedEmail => task.flagged_email,
            Self::List(id) => task.list_id == *id && !task.is_planner() && !task.flagged_email,
            Self::Search(query) => matches_search(task, query),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    #[default]
    Manual,
    Importance,
    DueDate,
    MyDay,
    Alphabetical,
    Created,
}

impl SortOrder {
    pub fn label(self) -> &'static str {
        match self {
            Self::Manual => "My order",
            Self::Importance => "Importance",
            Self::DueDate => "Due date",
            Self::MyDay => "Added to My Day",
            Self::Alphabetical => "Alphabetically",
            Self::Created => "Creation date",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TaskCount {
    pub active: usize,
    pub completed: usize,
}

impl TaskCount {
    pub fn total(self) -> usize {
        self.active + self.completed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlannedGroup {
    Earlier,
    Today,
    Tomorrow,
    ThisWeek,
    Later,
    NoDate,
}

impl AppState {
    /// Reverse each completion section independently, retaining the completed section last.
    /// The saved manual positions are never rewritten when a sort is applied or removed.
    pub fn visible_tasks_with_direction(
        &self,
        view: &View,
        sort: SortOrder,
        reversed: bool,
        show_completed: bool,
        today: Date,
    ) -> Vec<&Task> {
        let mut tasks = self.visible_tasks(view, sort, show_completed, today);
        if reversed && sort != SortOrder::Manual {
            let completed = tasks.partition_point(|task| !task.completed);
            tasks[..completed].reverse();
            tasks[completed..].reverse();
        }
        tasks
    }

    /// Filtering returns references and sorting never copies notes, steps, or attachments.
    pub fn visible_tasks(
        &self,
        view: &View,
        sort: SortOrder,
        show_completed: bool,
        today: Date,
    ) -> Vec<&Task> {
        let mut tasks: Vec<_> = self
            .tasks
            .iter()
            .filter(|task| {
                (!task.is_planner() || self.settings.planner_enabled)
                    && view.matches(task, today)
                    && (show_completed || !task.completed || *view == View::Completed)
            })
            .collect();
        if sort == SortOrder::Alphabetical {
            tasks.sort_by_cached_key(|task| (task.completed, task.title.to_lowercase(), task.id));
            return tasks;
        }
        tasks.sort_by(|a, b| {
            a.completed
                .cmp(&b.completed)
                .then_with(|| match sort {
                    SortOrder::Manual => a.position.cmp(&b.position),
                    SortOrder::Importance => b
                        .important
                        .cmp(&a.important)
                        .then(a.position.cmp(&b.position)),
                    SortOrder::DueDate => {
                        let date = |task: &Task| {
                            task.due_date.or_else(|| {
                                if *view == View::Planned {
                                    task.reminder.as_ref().map(|reminder| reminder.date)
                                } else {
                                    None
                                }
                            })
                        };
                        let (a, b) = (date(a), date(b));
                        a.is_none().cmp(&b.is_none()).then(a.cmp(&b))
                    }
                    SortOrder::MyDay => b
                        .is_my_day(today)
                        .cmp(&a.is_my_day(today))
                        .then(a.position.cmp(&b.position)),
                    SortOrder::Alphabetical => {
                        unreachable!("alphabetical sorting uses cached keys")
                    }
                    SortOrder::Created => b.id.cmp(&a.id),
                })
                .then(a.id.cmp(&b.id))
        });
        tasks
    }

    pub fn count(&self, view: &View, today: Date) -> TaskCount {
        self.tasks
            .iter()
            .filter(|task| {
                (!task.is_planner() || self.settings.planner_enabled) && view.matches(task, today)
            })
            .fold(TaskCount::default(), |mut count, task| {
                if task.completed {
                    count.completed += 1;
                } else {
                    count.active += 1;
                }
                count
            })
    }

    pub fn suggestions(&self, today: Date) -> Vec<&Task> {
        let mut tasks: Vec<_> = self
            .tasks
            .iter()
            .filter(|task| {
                (!task.is_planner() || self.settings.planner_enabled)
                    && !task.completed
                    && !task.is_my_day(today)
            })
            .collect();
        tasks.sort_by_key(|task| {
            (
                !task.is_overdue(today),
                !task.important,
                task.due_date.is_none(),
                task.due_date,
                task.id,
            )
        });
        tasks
    }

    pub fn reminders_due(&self, today: Date, minute: u16) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|task| {
                !task.completed
                    && task.reminder.as_ref().is_some_and(|reminder| {
                        !reminder.notified
                            && (reminder.date < today
                                || (reminder.date == today
                                    && reminder.minutes_after_midnight <= minute))
                    })
            })
            .collect()
    }
}

pub fn planned_group(task: &Task, today: Date) -> PlannedGroup {
    let date = task
        .due_date
        .or_else(|| task.reminder.as_ref().map(|reminder| reminder.date));
    let Some(date) = date else {
        return PlannedGroup::NoDate;
    };
    if date < today {
        return PlannedGroup::Earlier;
    }
    if date == today {
        return PlannedGroup::Today;
    }
    if Some(date) == today.succ_opt() {
        return PlannedGroup::Tomorrow;
    }
    let remaining = 6 - today.weekday().num_days_from_monday();
    if today
        .checked_add_signed(Duration::days(i64::from(remaining)))
        .is_some_and(|end| date <= end)
    {
        PlannedGroup::ThisWeek
    } else {
        PlannedGroup::Later
    }
}

fn matches_search(task: &Task, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return false;
    }
    let haystack = format!(
        "{} {} {} {}",
        task.title,
        task.notes,
        task.steps
            .iter()
            .map(|step| step.title.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        task.tags
            .iter()
            .map(|tag| format!("#{tag}"))
            .collect::<Vec<_>>()
            .join(" ")
    )
    .to_lowercase();
    query.split_whitespace().all(|word| haystack.contains(word))
}
