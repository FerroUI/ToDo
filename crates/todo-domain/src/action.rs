use crate::{Attachment, Date, Id, Recurrence, Reminder, Settings, SortOrder};
use serde::{Deserialize, Serialize};

/// Serializable commands shared by all hosts, keyboard actions, and import adapters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    AddTask {
        title: String,
        list_id: Id,
    },
    AddTaskInView {
        title: String,
        view: crate::View,
    },
    /// Create a task with parsed scheduling fields in one undoable transaction.
    AddTaskEntry {
        entry: crate::TaskEntry,
        view: crate::View,
    },
    RenameTask {
        id: Id,
        title: String,
    },
    SetNotes {
        id: Id,
        notes: String,
    },
    ToggleCompleted(Id),
    SetCompleted {
        id: Id,
        completed: bool,
    },
    ToggleImportant(Id),
    SetImportant {
        id: Id,
        important: bool,
    },
    SetMyDay {
        id: Id,
        day: Option<Date>,
    },
    SetDueDate {
        id: Id,
        date: Option<Date>,
    },
    SetReminder {
        id: Id,
        reminder: Option<Reminder>,
    },
    SetRecurrence {
        id: Id,
        recurrence: Option<Recurrence>,
    },
    SetTags {
        id: Id,
        tags: Vec<String>,
    },
    AssignTask {
        id: Id,
        assignee: Option<String>,
    },
    MoveTask {
        id: Id,
        list_id: Id,
    },
    ReorderTask {
        id: Id,
        before: Option<Id>,
    },
    DuplicateTask(Id),
    DeleteTask(Id),
    AddStep {
        task_id: Id,
        title: String,
    },
    RenameStep {
        task_id: Id,
        step_id: Id,
        title: String,
    },
    ToggleStep {
        task_id: Id,
        step_id: Id,
    },
    DeleteStep {
        task_id: Id,
        step_id: Id,
    },
    PromoteStep {
        task_id: Id,
        step_id: Id,
    },
    AddAttachment {
        task_id: Id,
        attachment: Attachment,
    },
    DeleteAttachment {
        task_id: Id,
        attachment_id: Id,
    },
    AddList {
        name: String,
    },
    RenameList {
        id: Id,
        name: String,
    },
    DeleteList(Id),
    DuplicateList(Id),
    SetListTheme {
        id: Id,
        color: String,
        background: Option<String>,
    },
    SetListSort {
        id: Id,
        sort: SortOrder,
    },
    SetListShowCompleted {
        id: Id,
        show: bool,
    },
    MoveList {
        id: Id,
        group_id: Option<Id>,
    },
    ReorderList {
        id: Id,
        before: Option<Id>,
    },
    AddGroup {
        name: String,
    },
    RenameGroup {
        id: Id,
        name: String,
    },
    DeleteGroup(Id),
    ToggleGroup(Id),
    DeleteCompleted {
        list_id: Option<Id>,
    },
    SetSettings(Settings),
    /// Imported commands execute in order as one atomic transaction.
    Batch(Vec<Action>),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changes {
    pub changed: bool,
    pub changed_tasks: Vec<Id>,
    pub deleted_tasks: Vec<Id>,
    pub created_task: Option<Id>,
    pub created_list: Option<Id>,
    pub created_group: Option<Id>,
    pub lists_changed: bool,
    pub settings_changed: bool,
}

impl Changes {
    pub(crate) fn task(id: Id) -> Self {
        Self {
            changed: true,
            changed_tasks: vec![id],
            ..Self::default()
        }
    }

    pub(crate) fn lists() -> Self {
        Self {
            changed: true,
            lists_changed: true,
            ..Self::default()
        }
    }

    pub(crate) fn merge(&mut self, other: Self) {
        self.changed |= other.changed;
        self.changed_tasks.extend(other.changed_tasks);
        self.deleted_tasks.extend(other.deleted_tasks);
        self.created_task = other.created_task.or(self.created_task);
        self.created_list = other.created_list.or(self.created_list);
        self.created_group = other.created_group.or(self.created_group);
        self.lists_changed |= other.lists_changed;
        self.settings_changed |= other.settings_changed;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DomainError {
    NotFound(&'static str, Id),
    EmptyName,
    InvalidValue(String),
    ProtectedList,
    InvalidData(String),
    FutureSchema(u32),
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(kind, id) => write!(f, "{kind} {id} no longer exists"),
            Self::EmptyName => f.write_str("Please enter a name"),
            Self::InvalidValue(message) | Self::InvalidData(message) => f.write_str(message),
            Self::ProtectedList => f.write_str("The default Tasks list cannot be changed"),
            Self::FutureSchema(version) => write!(f, "This backup uses newer format {version}"),
        }
    }
}

impl std::error::Error for DomainError {}

pub(crate) fn nonempty(value: String) -> Result<String, DomainError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        Err(DomainError::EmptyName)
    } else {
        Ok(value)
    }
}
