use crate::{Date, Id, Recurrence, SCHEMA_VERSION, TASKS_LIST};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: Id,
    pub list_id: Id,
    pub title: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub important: bool,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub my_day: Option<Date>,
    #[serde(default)]
    pub my_day_excluded: Option<Date>,
    #[serde(default)]
    pub due_date: Option<Date>,
    #[serde(default)]
    pub reminder: Option<Reminder>,
    #[serde(default)]
    pub recurrence: Option<Recurrence>,
    #[serde(default)]
    pub recurrence_generated: bool,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub assigned_to: Option<String>,
    #[serde(default)]
    pub flagged_email: bool,
    pub created_on: Date,
    pub updated_on: Date,
    #[serde(default)]
    pub completed_on: Option<Date>,
    #[serde(default)]
    pub position: i64,
}

impl Task {
    pub fn new(id: Id, list_id: Id, title: String, today: Date) -> Self {
        Self {
            id,
            list_id,
            title,
            notes: String::new(),
            important: false,
            completed: false,
            my_day: None,
            my_day_excluded: None,
            due_date: None,
            reminder: None,
            recurrence: None,
            recurrence_generated: false,
            steps: Vec::new(),
            attachments: Vec::new(),
            tags: Vec::new(),
            assigned_to: None,
            flagged_email: false,
            created_on: today,
            updated_on: today,
            completed_on: None,
            position: id as i64,
        }
    }

    pub fn completed_steps(&self) -> usize {
        self.steps.iter().filter(|step| step.completed).count()
    }

    pub fn is_overdue(&self, today: Date) -> bool {
        !self.completed && self.due_date.is_some_and(|due| due < today)
    }

    pub fn is_my_day(&self, today: Date) -> bool {
        self.my_day == Some(today)
            || (self.due_date == Some(today)
                && self.my_day_excluded != Some(today)
                && (!self.completed || self.completed_on == Some(today)))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    pub id: Id,
    pub title: String,
    #[serde(default)]
    pub completed: bool,
}

/// Browser attachments live in IndexedDB. Only their stable key and metadata enter JSON.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    pub id: Id,
    pub name: String,
    pub mime_type: String,
    pub size: u64,
    pub storage_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reminder {
    pub date: Date,
    /// Local civil time, 0..1440. The platform resolves timezone/DST at delivery time.
    pub minutes_after_midnight: u16,
    #[serde(default)]
    pub notified: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskList {
    pub id: Id,
    pub name: String,
    #[serde(default = "default_list_color")]
    pub color: String,
    #[serde(default)]
    pub background: Option<String>,
    #[serde(default)]
    pub group_id: Option<Id>,
    #[serde(default)]
    pub position: i64,
    #[serde(default)]
    pub sort: crate::SortOrder,
    #[serde(default = "default_true")]
    pub show_completed: bool,
}

impl TaskList {
    pub fn new(id: Id, name: String) -> Self {
        Self {
            id,
            name,
            color: default_list_color(),
            background: None,
            group_id: None,
            position: id as i64,
            sort: crate::SortOrder::Manual,
            show_completed: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListGroup {
    pub id: Id,
    pub name: String,
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default)]
    pub position: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: Theme,
    pub confirm_delete: bool,
    pub completion_sound: bool,
    pub show_completed: bool,
    pub show_all: bool,
    pub show_completed_list: bool,
    pub show_assigned: bool,
    pub show_flagged_email: bool,
    pub week_starts_monday: bool,
    pub recognize_dates: bool,
    pub remove_recognized_dates: bool,
    pub account_name: String,
    pub microsoft_client_id: String,
    pub smart_colors: std::collections::BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            confirm_delete: true,
            completion_sound: true,
            show_completed: true,
            show_all: true,
            show_completed_list: false,
            show_assigned: false,
            show_flagged_email: false,
            week_starts_monday: true,
            recognize_dates: true,
            remove_recognized_dates: false,
            account_name: "My account".into(),
            microsoft_client_id: String::new(),
            smart_colors: std::collections::BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppState {
    pub schema_version: u32,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub lists: Vec<TaskList>,
    #[serde(default)]
    pub groups: Vec<ListGroup>,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default = "initial_id")]
    pub next_id: Id,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            tasks: Vec::new(),
            lists: vec![TaskList::new(TASKS_LIST, "Tasks".into())],
            groups: Vec::new(),
            settings: Settings::default(),
            next_id: initial_id(),
        }
    }

    pub fn task(&self, id: Id) -> Option<&Task> {
        self.tasks.iter().find(|task| task.id == id)
    }
    pub fn list(&self, id: Id) -> Option<&TaskList> {
        self.lists.iter().find(|list| list.id == id)
    }
    pub fn group(&self, id: Id) -> Option<&ListGroup> {
        self.groups.iter().find(|group| group.id == id)
    }
    pub(crate) fn allocate_id(&mut self) -> Id {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("entity identifier space exhausted");
        id
    }
}

fn default_list_color() -> String {
    "#3E69E4".into()
}
fn default_true() -> bool {
    true
}
fn initial_id() -> Id {
    2
}
