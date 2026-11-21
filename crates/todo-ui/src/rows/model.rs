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

impl Row {
    pub(super) fn same_key(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Task { id: a, .. }, Self::Task { id: b, .. }) => a == b,
            (Self::Header { key: a, .. }, Self::Header { key: b, .. }) => a == b,
            _ => false,
        }
    }
    pub(super) fn boxed(&self) -> Option<BoxedValue> {
        Some(Rc::new(self.clone()))
    }
}
