//! In-app task drags use a private typed payload, never external text or file data.
mod cancellation;
mod feedback;
mod source;
mod target;

pub(crate) use feedback::marker;
pub(crate) use source::{attach_source, ignore_click};
pub(crate) use target::{attach_list_target, attach_task_target};

use ferroui_base::input::{DataFormat, DataFormatOf, DataTransferExtensions, DragEventArgs};
use ferroui_base::BoxedValue;
use ferroui_controls::Control;
use std::cell::Cell;
use std::rc::{Rc, Weak};
use todo_domain::{Action, Id, SortOrder, View};
use todo_state::{AppViewModel, Panel};

type Identity = fn(&Control) -> Option<Id>;

#[derive(Clone)]
struct TaskDrag {
    task_id: Id,
    owner: Weak<AppViewModel>,
    active: Rc<Cell<bool>>,
}

fn format() -> DataFormatOf<TaskDrag> {
    DataFormat::create_in_process_format("FerroUI.ToDo.Task")
}

fn payload(event: &DragEventArgs, vm: &Rc<AppViewModel>) -> Option<TaskDrag> {
    let drag = event.data_transfer().try_get_value(&format())?;
    (drag.active.get()
        && drag.owner.ptr_eq(&Rc::downgrade(vm))
        && !vm.selection_mode()
        && vm.panel() == Panel::None
        && vm.state().task(drag.task_id).is_some())
    .then_some(drag)
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct ListIdentity(Id);

pub(crate) fn list_tag(view: &View) -> Option<BoxedValue> {
    match view {
        View::List(id) => Some(Rc::new(ListIdentity(*id))),
        _ => None,
    }
}

fn list_id(control: &Control) -> Option<Id> {
    control
        .tag()
        .and_then(|tag| tag.downcast_ref::<ListIdentity>().map(|id| id.0))
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
