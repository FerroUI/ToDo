//! A drag keeps its original source meaning and cannot resume after invalidation.
use crate::{AppViewModel, PaneLayout, Panel};
use std::cell::Cell;
use todo_domain::{Id, SidebarItem, SortOrder, View, TASKS_LIST};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragItem {
    Task(Id),
    Sidebar(SidebarItem),
}

impl DragItem {
    pub fn exists(self, vm: &AppViewModel) -> bool {
        let state = vm.state();
        match self {
            Self::Task(id) => state.task(id).is_some(),
            Self::Sidebar(SidebarItem::List(id)) => id != TASKS_LIST && state.list(id).is_some(),
            Self::Sidebar(SidebarItem::Group(id)) => state.group(id).is_some(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragInput {
    Mouse,
    Touch,
}

pub struct DragOrigin {
    pub item: DragItem,
    input: DragInput,
    view: View,
    parent: Option<Id>,
    completed: Option<bool>,
    sort: SortOrder,
    reversed: bool,
    width: f64,
    layout: PaneLayout,
    selected: Option<Id>,
    live: Cell<bool>,
}

impl DragOrigin {
    pub fn capture(vm: &AppViewModel, item: DragItem, input: DragInput) -> Option<Self> {
        let (parent, completed) = match item {
            DragItem::Task(id) => {
                let state = vm.state();
                let task = state.task(id)?;
                if !task.can_move()
                    || !movable_selection(vm)
                    || input == DragInput::Touch
                        && (vm.view() != View::List(task.list_id) || vm.sort() != SortOrder::Manual)
                {
                    return None;
                }
                (Some(task.list_id), Some(task.completed))
            }
            DragItem::Sidebar(SidebarItem::List(id)) => (vm.state().list(id)?.group_id, None),
            DragItem::Sidebar(SidebarItem::Group(_)) => (None, None),
        };
        let origin = Self {
            item,
            input,
            view: vm.view(),
            parent,
            completed,
            sort: vm.sort(),
            reversed: vm.sort_reversed(),
            width: vm.viewport(),
            layout: vm.layout(),
            selected: vm.selected(),
            live: Cell::new(true),
        };
        origin.valid(vm).then_some(origin)
    }

    pub fn active(&self) -> bool {
        self.live.get()
    }

    /// Returns whether this call consumed the still-live operation, at most once.
    pub fn finish(&self) -> bool {
        self.live.replace(false)
    }

    /// Call on model changes as well as at drop time. Returning to an earlier route or
    /// undoing a remote move must not revive a contact that was already interrupted.
    pub fn valid(&self, vm: &AppViewModel) -> bool {
        if self.live.get() && !self.matches(vm) {
            self.live.set(false);
        }
        self.live.get()
    }

    fn matches(&self, vm: &AppViewModel) -> bool {
        if vm.view() != self.view
            || vm.panel() != Panel::None
            || vm.rename_session().is_some()
            || vm.refreshing()
            || vm.viewport() != self.width
            || vm.sort() != self.sort
            || vm.sort_reversed() != self.reversed
            || !self.item.exists(vm)
        {
            return false;
        }
        let held_task = self.input == DragInput::Touch && matches!(self.item, DragItem::Task(_));
        // A task hold enters selection and closes details itself. Every other drag keeps
        // its original pane topology and selection; recycling individual rows is harmless.
        if !held_task
            && (vm.selection_mode() || vm.selected() != self.selected || vm.layout() != self.layout)
        {
            return false;
        }
        match self.item {
            DragItem::Task(id) => vm.state().task(id).is_some_and(|task| {
                task.can_move()
                    && movable_selection(vm)
                    && Some(task.list_id) == self.parent
                    && Some(task.completed) == self.completed
            }),
            DragItem::Sidebar(SidebarItem::List(id)) => vm
                .state()
                .list(id)
                .is_some_and(|list| list.group_id == self.parent),
            DragItem::Sidebar(SidebarItem::Group(_)) => true,
        }
    }

    pub fn label(&self) -> &'static str {
        match self.item {
            DragItem::Task(_) => "Move task",
            DragItem::Sidebar(SidebarItem::List(_)) => "Move list",
            DragItem::Sidebar(SidebarItem::Group(_)) => "Move group",
        }
    }
}

fn movable_selection(vm: &AppViewModel) -> bool {
    let state = vm.state();
    vm.selection()
        .iter()
        .all(|id| state.task(*id).is_some_and(|task| task.can_move()))
}
