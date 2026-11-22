use ferroui_base::BoxedValue;
use std::rc::Rc;
use todo_domain::Id;
/// Only the render projection enters the item source; large notes and attachment metadata stay in MVVM.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    Task {
        id: Id,
        title: String,
        completed: bool,
        important: bool,
        overdue: bool,
        metadata: String,
        selected: bool,
        selection_mode: bool,
        multi_selected: bool,
        dark: bool,
        accent: String,
    },
    Header {
        key: &'static str,
        label: String,
        completed: bool,
        expanded: bool,
        foreground: &'static str,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Key {
    Task(Id),
    Header(&'static str),
}

impl Row {
    pub(super) fn key(&self) -> Key {
        match self {
            Self::Task { id, .. } => Key::Task(*id),
            Self::Header { key, .. } => Key::Header(key),
        }
    }

    pub(super) fn same_key(&self, other: &Self) -> bool {
        self.key() == other.key()
    }
    pub(super) fn boxed(&self) -> Option<BoxedValue> {
        Some(Rc::new(self.clone()))
    }
}
