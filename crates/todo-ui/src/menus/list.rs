use super::item::{command, destructive, submenu};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::{Action, Id, TASKS_LIST};
use todo_state::{AppViewModel, Panel, RenameTarget};

/// Explicit identity: right-clicking another list must not act on the active one.
pub(super) fn items(vm: Rc<AppViewModel>, id: Id) -> Vec<Element> {
    if vm.state().list(id).is_none() {
        return Vec::new();
    }
    let duplicate = vm.clone();
    let mut items = vec![command("Duplicate list", Icon::Copy, move || {
        duplicate.dispatch(Action::DuplicateList(id));
    })];
    if id == TASKS_LIST {
        return items;
    }
    let rename = vm.clone();
    items.insert(
        0,
        command("Rename list", Icon::Note, move || {
            rename.begin_rename(RenameTarget::List(id));
        }),
    );
    items.push(submenu(
        "Move to group",
        Icon::Folder,
        destinations(vm.clone(), id),
    ));
    if vm
        .state()
        .list(id)
        .is_some_and(|list| list.group_id.is_some())
    {
        let remove = vm.clone();
        items.push(command("Remove from group", Icon::List, move || {
            remove.dispatch(Action::MoveList { id, group_id: None });
        }));
    }
    let up = vm.clone();
    let down = vm.clone();
    items.push(command("Move up", Icon::ArrowUp, move || {
        up.move_list_by(id, -1)
    }));
    items.push(command("Move down", Icon::ArrowDown, move || {
        down.move_list_by(id, 1)
    }));
    items.push(separator().into());
    items.push(destructive(
        "Delete list",
        Icon::Trash,
        vm.is_dark(),
        move || vm.show_panel(Panel::DeleteList(id)),
    ));
    items
}

fn destinations(vm: Rc<AppViewModel>, id: Id) -> Vec<Element> {
    let state = vm.state();
    let current = state.list(id).and_then(|list| list.group_id);
    let mut groups = state.groups.clone();
    groups.sort_by_key(|group| (group.position, group.id));
    drop(state);
    let mut items = Vec::new();
    if current.is_some() {
        let model = vm.clone();
        items.push(command("Outside a group", Icon::List, move || {
            model.dispatch(Action::MoveList { id, group_id: None });
        }));
    }
    for group in groups.into_iter().filter(|group| Some(group.id) != current) {
        let model = vm.clone();
        let group_id = Some(group.id);
        items.push(command(group.name, Icon::Folder, move || {
            model.dispatch(Action::MoveList { id, group_id });
        }));
    }
    items
}
