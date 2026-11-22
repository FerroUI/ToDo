//! Task and navigation drags share one gesture engine and private typed in-process payload.
mod cancellation;
mod feedback;
mod sidebar;
mod sidebar_feedback;
mod source;
mod target;
mod touch;

pub(crate) use feedback::marker;
pub(crate) use sidebar::{SidebarTarget, attach_sidebar_target, sidebar_marker};
pub(crate) use source::attach_sidebar_source;
pub(crate) use source::{attach_source, ignore_click};
pub(crate) use target::{attach_list_target, attach_task_target};
pub(crate) use touch::TouchDrag;

use ferroui_base::BoxedValue;
use ferroui_base::input::{DataFormat, DataFormatOf, DataTransferExtensions, DragEventArgs};
use ferroui_controls::Control;
use std::cell::Cell;
use std::rc::{Rc, Weak};
use todo_domain::{Action, Id, SidebarItem, SortOrder, TASKS_LIST, View};
use todo_state::{AppViewModel, Panel};

type Identity = fn(&Control) -> Option<Id>;

#[derive(Clone)]
struct ItemDrag {
    item: DragItem,
    owner: Weak<AppViewModel>,
    active: Rc<Cell<bool>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DragItem {
    Task(Id),
    Sidebar(SidebarItem),
}

impl DragItem {
    fn exists(self, vm: &AppViewModel) -> bool {
        let state = vm.state();
        match self {
            Self::Task(id) => state.task(id).is_some(),
            Self::Sidebar(SidebarItem::List(id)) => id != TASKS_LIST && state.list(id).is_some(),
            Self::Sidebar(SidebarItem::Group(id)) => state.group(id).is_some(),
        }
    }
}

fn format() -> DataFormatOf<ItemDrag> {
    DataFormat::create_in_process_format("FerroUI.ToDo.Item")
}

fn payload(event: &DragEventArgs, vm: &Rc<AppViewModel>) -> Option<ItemDrag> {
    let drag = event.data_transfer().try_get_value(&format())?;
    (drag.active.get()
        && drag.owner.ptr_eq(&Rc::downgrade(vm))
        && !vm.selection_mode()
        && vm.panel() == Panel::None
        && vm.rename_session().is_none()
        && drag.item.exists(vm))
    .then_some(drag)
}

pub(crate) fn list_tag(view: &View) -> Option<BoxedValue> {
    match view {
        View::List(id) => Some(Rc::new(SidebarItem::List(*id))),
        _ => None,
    }
}

fn list_id(control: &Control) -> Option<Id> {
    match sidebar_identity(control)? {
        SidebarItem::List(id) => Some(id),
        _ => None,
    }
}

pub(crate) fn sidebar_identity(control: &Control) -> Option<SidebarItem> {
    control
        .tag()
        .and_then(|tag| tag.downcast_ref::<SidebarItem>().copied())
}

fn task_payload(event: &DragEventArgs, vm: &Rc<AppViewModel>) -> Option<(ItemDrag, Id)> {
    let Some(drag) = payload(event, vm) else {
        event.set_drag_effects(ferroui_base::input::DragDropEffects::NONE);
        return None;
    };
    let DragItem::Task(id) = drag.item else {
        // A later sidebar handler on the same list button can accept its own payload.
        event.set_drag_effects(ferroui_base::input::DragDropEffects::NONE);
        return None;
    };
    Some((drag, id))
}

/// Compute against the live document at drop time; hover never changes task state.
fn reorder(vm: &AppViewModel, id: Id, target: Id, after: bool) -> Option<Action> {
    if id == target || vm.sort() != SortOrder::Manual {
        return None;
    }
    let View::List(list_id) = vm.view() else {
        return None;
    };
    let document = vm.state();
    let source = document.task(id)?;
    let target = document.task(target)?;
    if source.list_id != list_id
        || target.list_id != list_id
        || source.completed != target.completed
    {
        return None;
    }
    let before = if after {
        document
            .tasks
            .iter()
            .filter(|candidate| {
                candidate.id != id
                    && candidate.list_id == list_id
                    && candidate.completed == target.completed
                    && (candidate.position, candidate.id) > (target.position, target.id)
            })
            .min_by_key(|candidate| (candidate.position, candidate.id))
            .map(|candidate| candidate.id)
    } else {
        Some(target.id)
    };
    Some(Action::ReorderTask { id, before })
}

fn move_to(vm: &AppViewModel, id: Id, list_id: Id) -> Option<Action> {
    let document = vm.state();
    if document.task(id)?.list_id == list_id || document.list(list_id).is_none() {
        return None;
    }
    Some(Action::MoveTask { id, list_id })
}
