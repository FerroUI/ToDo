mod diff;
mod model;
pub use model::Row;

use ferroui_base::{BoxedValue, collections::FerroList};
use ferroui_controls::ItemsSource;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use todo_domain::{Id, Task, View};
use todo_state::AppViewModel;

/// A stable observable source with one recycling data template for the view's lifetime.
pub struct Rows {
    data: RefCell<Rc<Vec<Row>>>,
    pending: RefCell<Option<Rc<Vec<Row>>>>,
    applying: Cell<bool>,
    items: Rc<FerroList<Option<BoxedValue>>>,
    pub template: Rc<dyn ferroui_controls::templates::IDataTemplate>,
}

impl Rows {
    pub fn new(vm: Rc<AppViewModel>) -> Self {
        Self {
            data: RefCell::new(Rc::new(Vec::new())),
            pending: RefCell::new(None),
            applying: Cell::new(false),
            items: Rc::new(FerroList::new()),
            template: ferroui_view::template(move |row: Row| crate::list::row(vm.clone(), row)),
        }
    }
    pub fn source(&self) -> ItemsSource {
        self.items.clone().into()
    }
    pub fn is_empty(&self) -> bool {
        self.data.borrow().is_empty()
    }
    pub fn task_ids(&self) -> Vec<Id> {
        self.data
            .borrow()
            .iter()
            .filter_map(|row| {
                if let Row::Task { id, .. } = row {
                    Some(*id)
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn index_of(&self, target: Id) -> Option<i32> {
        self.data
            .borrow()
            .iter()
            .position(|row| matches!(row, Row::Task { id, .. } if *id == target))
            .and_then(|index| i32::try_from(index).ok())
    }

    pub fn synchronize(&self, vm: &AppViewModel) {
        let view = vm.view();
        let today = vm.today();
        let dark = vm.is_dark();
        let accent = crate::list::list_accent(vm);
        let selection = vm.selection();
        let cursor = vm.task_cursor();
        let foreground = if view == View::MyDay {
            "#FFFFFF"
        } else {
            todo_controls::Palette::for_dark(dark).text
        };
        let state = vm.state();
        let show_completed = match &view {
            View::List(id) => state.list(*id).is_some_and(|list| list.show_completed),
            _ => state.settings.show_completed,
        };
        let mut tasks = state.visible_tasks_with_direction(
            &view,
            vm.sort(),
            vm.sort_reversed(),
            show_completed,
            today,
        );
        if view == View::Planned {
            // Keep date groups contiguous even when the user changes the secondary sort.
            tasks.sort_by_key(|task| {
                let date = task
                    .due_date
                    .or_else(|| task.reminder.as_ref().map(|r| r.date));
                let rank = match date {
                    Some(date) if date < today => 0,
                    Some(date) if date == today => 1,
                    Some(date) if Some(date) == today.succ_opt() => 2,
                    Some(_) => 3,
                    None => 4,
                };
                (task.completed, rank)
            });
        }
        let mut next = Vec::with_capacity(tasks.len() + 6);
        let mut group = "";
        for task in tasks.iter().filter(|task| !task.completed) {
            if view == View::Planned {
                let date = task
                    .due_date
                    .or_else(|| task.reminder.as_ref().map(|r| r.date));
                let label = match date {
                    Some(date) if date < today => "Earlier",
                    Some(date) if date == today => "Today",
                    Some(date) if Some(date) == today.succ_opt() => "Tomorrow",
                    Some(_) => "Later",
                    None => "Reminders",
                };
                if label != group {
                    group = label;
                    next.push(Row::Header {
                        key: label,
                        label: label.into(),
                        completed: false,
                        expanded: true,
                        foreground,
                    });
                }
            }
            next.push(Self::task(vm, task, dark, &selection, &accent, cursor));
        }
        let completed = tasks.iter().filter(|task| task.completed).count();
        if completed > 0 {
            if view != View::Completed {
                next.push(Row::Header {
                    key: "completed",
                    label: format!("Completed  {completed}"),
                    completed: true,
                    expanded: vm.completed_expanded(),
                    foreground,
                });
            }
            if view == View::Completed || vm.completed_expanded() {
                next.extend(
                    tasks
                        .iter()
                        .filter(|task| task.completed)
                        .map(|task| Self::task(vm, task, dark, &selection, &accent, cursor)),
                );
            }
        }
        // Source notifications may call into the view model. Release the document first.
        drop(tasks);
        drop(state);
        self.apply(next);
    }

    fn apply(&self, next: Vec<Row>) {
        *self.pending.borrow_mut() = Some(Rc::new(next));
        if self.applying.replace(true) {
            return;
        }
        let _guard = ApplyingGuard(&self.applying);
        loop {
            let next = self.pending.borrow_mut().take();
            let Some(next) = next else { break };
            // Publish the immutable projection before notifying listeners; no RefCell borrow
            // survives collection callbacks. A nested update is drained after this diff.
            let previous = self.data.replace(next.clone());
            let previous = Rc::try_unwrap(previous).unwrap_or_else(|shared| (*shared).clone());
            diff::reconcile(&self.items, previous, &next);
        }
    }

    fn task(
        vm: &AppViewModel,
        task: &Task,
        dark: bool,
        selection: &std::collections::BTreeSet<Id>,
        accent: &str,
        cursor: Option<Id>,
    ) -> Row {
        Row::Task {
            id: task.id,
            title: task.title.clone(),
            completed: task.completed,
            important: task.important,
            overdue: task.is_overdue(vm.today()),
            metadata: vm.task_metadata(task),
            selected: vm.selected() == Some(task.id)
                || cursor == Some(task.id)
                || selection.contains(&task.id),
            selection_mode: vm.selection_mode(),
            multi_selected: selection.contains(&task.id),
            dark,
            accent: accent.into(),
        }
    }
}

struct ApplyingGuard<'a>(&'a Cell<bool>);
impl Drop for ApplyingGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
