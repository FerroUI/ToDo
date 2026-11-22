use super::item::{command, destructive, submenu};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::{Action, Id, TASKS_LIST};
use todo_state::{AppViewModel, RenameTarget};

pub(super) fn items(vm: Rc<AppViewModel>, id: Id) -> Vec<Element> {
    if !vm.state().groups.iter().any(|group| group.id == id) {
        return Vec::new();
    }
    let rename = vm.clone();
    let up = vm.clone();
    let down = vm.clone();
    let mut lists = vm.state().lists.clone();
    let occupied = lists.iter().any(|list| list.group_id == Some(id));
    lists.sort_by_key(|list| (list.position, list.id));
    let destinations = lists
        .into_iter()
        .filter(|list| list.id != TASKS_LIST && list.group_id != Some(id))
        .map(|list| {
            let add = vm.clone();
            command(list.name, Icon::List, move || {
                add.dispatch(Action::MoveList {
                    id: list.id,
                    group_id: Some(id),
                })
            })
        })
        .collect();
    let remove = if occupied {
        command("Ungroup lists", Icon::Folder, move || {
            vm.dispatch(Action::DeleteGroup(id))
        })
    } else {
        destructive("Delete group", Icon::Trash, vm.is_dark(), move || {
            vm.dispatch(Action::DeleteGroup(id))
        })
    };
    vec![
        command("Rename group", Icon::Note, move || {
            rename.begin_rename(RenameTarget::Group(id))
        }),
        submenu("Add list to group", Icon::Add, destinations),
        command("Move up", Icon::ArrowUp, move || up.move_group_by(id, -1)),
        command("Move down", Icon::ArrowDown, move || {
            down.move_group_by(id, 1)
        }),
        separator().into(),
        remove,
    ]
}
