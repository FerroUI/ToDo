//! A drag keeps its original source meaning and cannot resume after invalidation.
use crate::{AppViewModel, PaneLayout, Panel};
use std::cell::Cell;
use std::collections::BTreeSet;
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
    document_epoch: u64,
    view: View,
    parent: Option<Id>,
    tasks: Vec<(Id, Id, bool, i64)>,
    task_ids: Vec<Id>,
    selection: BTreeSet<Id>,
    held_selection: BTreeSet<Id>,
    selection_mode: bool,
    hold_selected: Cell<bool>,
    sort: SortOrder,
    reversed: bool,
    width: f64,
    layout: PaneLayout,
    selected: Option<Id>,
    live: Cell<bool>,
}

impl DragOrigin {
    pub fn capture(vm: &AppViewModel, item: DragItem, input: DragInput) -> Option<Self> {
        let selection = vm.selection();
        let mut held_selection = selection.clone();
        let mut task_ids = Vec::new();
        let mut tasks = Vec::new();
        let parent = match item {
            DragItem::Task(id) => {
                let state = vm.state();
                let task = state.task(id)?;
                if !task.can_move()
                    || input == DragInput::Touch
                        && (vm.view() != View::List(task.list_id) || vm.sort() != SortOrder::Manual)
                {
                    return None;
                }
                if input == DragInput::Touch {
                    held_selection.insert(id);
                    let mut remaining = held_selection.clone();
                    task_ids = vm
                        .visible_task_ids()
                        .into_iter()
                        .filter(|id| remaining.remove(id))
                        .collect();
                    task_ids.extend(
                        vm.ordered_selection()
                            .into_iter()
                            .filter(|id| remaining.remove(id)),
                    );
                    task_ids.extend(remaining);
                } else {
                    task_ids = vm.task_action_ids(Some(id));
                }
                for id in &task_ids {
                    let task = state.task(*id)?;
                    if !task.can_move() {
                        return None;
                    }
                    tasks.push((task.id, task.list_id, task.completed, task.position));
                }
                Some(task.list_id)
            }
            DragItem::Sidebar(SidebarItem::List(id)) => vm.state().list(id)?.group_id,
            DragItem::Sidebar(SidebarItem::Group(_)) => None,
        };
        let origin = Self {
            item,
            input,
            document_epoch: vm.clipboard_epoch.get(),
            view: vm.view(),
            parent,
            tasks,
            task_ids,
            selection,
            held_selection,
            selection_mode: vm.selection_mode(),
            hold_selected: Cell::new(false),
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

    pub fn task_ids(&self) -> &[Id] {
        &self.task_ids
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
        if vm.clipboard_epoch.get() != self.document_epoch
            || vm.view() != self.view
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
        if held_task {
            if vm.selection_mode()
                && vm.selected().is_none()
                && vm.selection() == self.held_selection
            {
                self.hold_selected.set(true);
            } else if self.hold_selected.get()
                || vm.selection_mode() != self.selection_mode
                || vm.selection() != self.selection
                || vm.selected() != self.selected
                || vm.layout() != self.layout
            {
                return false;
            }
        } else if vm.selection_mode() != self.selection_mode
            || vm.selection() != self.selection
            || vm.selected() != self.selected
            || vm.layout() != self.layout
        {
            return false;
        }
        match self.item {
            DragItem::Task(_) => {
                let state = vm.state();
                self.tasks.iter().all(|(id, parent, completed, position)| {
                    state.task(*id).is_some_and(|task| {
                        task.can_move()
                            && task.list_id == *parent
                            && task.completed == *completed
                            && task.position == *position
                    })
                })
            }
            DragItem::Sidebar(SidebarItem::List(id)) => vm
                .state()
                .list(id)
                .is_some_and(|list| !vm.selection_mode() && list.group_id == self.parent),
            DragItem::Sidebar(SidebarItem::Group(_)) => !vm.selection_mode(),
        }
    }

    pub fn label(&self) -> &'static str {
        match self.item {
            DragItem::Task(_) if self.task_ids.len() > 1 => "Move selected tasks",
            DragItem::Task(_) => "Move task",
            DragItem::Sidebar(SidebarItem::List(_)) => "Move list",
            DragItem::Sidebar(SidebarItem::Group(_)) => "Move group",
        }
    }
}
