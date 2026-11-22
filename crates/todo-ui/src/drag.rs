//! Task and navigation drags share one gesture engine and private typed in-process payload.
mod cancellation;
mod completion;
mod feedback;
mod native;
mod registry;
mod sidebar;
mod sidebar_feedback;
mod source;
mod target;
mod touch;
mod touch_plan;

pub(crate) use feedback::marker;
pub(crate) use registry::{cancel_active, has_active};
pub(crate) use sidebar::{attach_sidebar_target, sidebar_marker, SidebarTarget};
pub(crate) use source::attach_sidebar_source;
pub(crate) use source::{attach_source, ignore_click};
pub(crate) use target::{attach_list_target, attach_task_target};
pub(crate) use touch::TouchDrag;

use ferroui_base::input::{DataFormat, DataFormatOf, DataTransferExtensions, DragEventArgs};
use ferroui_base::BoxedValue;
use ferroui_controls::Control;
use std::rc::Rc;
use todo_domain::{Action, Id, SidebarItem, SortOrder, View};
use todo_state::gestures::drag::DragItem;
use todo_state::AppViewModel;

type Identity = fn(&Control) -> Option<Id>;

#[derive(Clone)]
struct ItemDrag {
    item: DragItem,
    session: Rc<native::NativeDrag>,
}

fn format() -> DataFormatOf<ItemDrag> {
    DataFormat::create_in_process_format("FerroUI.ToDo.Item")
}

fn payload(event: &DragEventArgs, vm: &Rc<AppViewModel>) -> Option<ItemDrag> {
    let drag = event.data_transfer().try_get_value(&format())?;
    drag.session.valid(vm).then_some(drag)
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
