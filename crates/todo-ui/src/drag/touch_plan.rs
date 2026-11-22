use super::{list_id, move_to, reorder, sidebar, sidebar_feedback::Placement, DragItem};
use ferroui_base::{Point, Ref};
use ferroui_controls::{Control, TopLevel};
use todo_domain::{Action, Id, SidebarItem, SortOrder, View};
use todo_state::{AppViewModel, Panel};

pub(super) struct Origin {
    pub item: DragItem,
    view: View,
    parent: Option<Id>,
}

impl Origin {
    pub fn capture(vm: &AppViewModel, item: DragItem) -> Option<Self> {
        let view = vm.view();
        let parent = match item {
            DragItem::Task(id) => {
                let View::List(list) = view else { return None };
                if vm.sort() != SortOrder::Manual || vm.state().task(id)?.list_id != list {
                    return None;
                }
                Some(list)
            }
            DragItem::Sidebar(SidebarItem::List(id)) => vm.state().list(id)?.group_id,
            DragItem::Sidebar(SidebarItem::Group(_)) => None,
        };
        let origin = Self { item, view, parent };
        origin.valid(vm).then_some(origin)
    }

    pub fn valid(&self, vm: &AppViewModel) -> bool {
        if vm.view() != self.view
            || vm.panel() != Panel::None
            || vm.rename_session().is_some()
            || !self.item.exists(vm)
        {
            return false;
        }
        // A remotely moved source must not acquire a different meaning midway through a drag.
        match self.item {
            DragItem::Task(id) => {
                vm.sort() == SortOrder::Manual
                    && vm
                        .state()
                        .task(id)
                        .is_some_and(|task| Some(task.list_id) == self.parent)
            }
            DragItem::Sidebar(SidebarItem::List(id)) => {
                !vm.selection_mode()
                    && vm
                        .state()
                        .list(id)
                        .is_some_and(|list| list.group_id == self.parent)
            }
            DragItem::Sidebar(SidebarItem::Group(_)) => !vm.selection_mode(),
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

#[derive(Clone, Copy)]
pub(super) enum Cue {
    Task(bool),
    Sidebar(Placement),
}

pub(super) struct Plan {
    pub control: Ref<Control>,
    pub action: Action,
    pub cue: Cue,
}

pub(super) fn plan(
    root: &TopLevel,
    vm: &AppViewModel,
    origin: &Origin,
    position: Point,
) -> Option<Plan> {
    if !origin.valid(vm) {
        return None;
    }
    let hit = root.input_hit_test_with(position, false)?;
    for control in hit
        .get_self_and_visual_ancestors()
        .filter_map(|visual| visual.cast::<Control>())
    {
        let point = root.translate_point(position, &control)?;
        let after = point.y >= control.bounds().height / 2.0;
        let planned = match origin.item {
            DragItem::Task(id) => {
                if let Some(target) = crate::touch::task_id(&control) {
                    reorder(vm, id, target, after).map(|action| (action, Cue::Task(after)))
                } else if let Some(list) = list_id(&control) {
                    move_to(vm, id, list).map(|action| (action, Cue::Task(false)))
                } else {
                    None
                }
            }
            DragItem::Sidebar(item) => sidebar::target(&control)
                .and_then(|target| sidebar::plan(&control, vm, item, target, point))
                .map(|(action, placement)| (action, Cue::Sidebar(placement))),
        };
        if let Some((action, cue)) = planned {
            return Some(Plan {
                control,
                action,
                cue,
            });
        }
    }
    None
}
