use super::item::{command, submenu};
use super::task_action::batch;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::{Action, Id, SortOrder, View};
use todo_state::AppViewModel;

pub(super) fn items(vm: Rc<AppViewModel>, id: Id) -> Vec<Element> {
    let selection = vm.selection();
    let selected = selection.contains(&id);
    let state = vm.state();
    if state.task(id).is_none() {
        return Vec::new();
    }
    let tasks: Vec<_> = state
        .tasks
        .iter()
        .filter(|task| {
            if selected {
                selection.contains(&task.id)
            } else {
                task.id == id
            }
        })
        .collect();
    let ids: Rc<[Id]> = tasks.iter().map(|task| task.id).collect();
    let count = ids.len();
    let included = tasks.iter().all(|task| task.is_my_day(vm.today()));
    let important = tasks.iter().all(|task| task.important);
    let completed = tasks.iter().all(|task| task.completed);
    let can_move = tasks.iter().all(|task| task.can_move());
    let can_delete = tasks.iter().all(|task| task.can_delete());
    let mut lists = state.lists.clone();
    lists.sort_by_key(|list| (list.position, list.id));
    let destinations: Vec<_> = lists
        .into_iter()
        .filter(|list| tasks.iter().any(|task| task.list_id != list.id))
        .collect();
    drop(state);
    let mut items = vec![
        batch(
            vm.clone(),
            ids.clone(),
            if included {
                "Remove from My Day"
            } else {
                "Add to My Day"
            },
            Icon::Sun,
            move |id, vm| Action::SetMyDay {
                id,
                day: (!included).then(|| vm.today()),
            },
        ),
        batch(
            vm.clone(),
            ids.clone(),
            if important {
                "Remove importance"
            } else {
                "Mark as important"
            },
            Icon::Star,
            move |id, _| Action::SetImportant {
                id,
                important: !important,
            },
        ),
        batch(
            vm.clone(),
            ids.clone(),
            if completed {
                "Mark as not completed"
            } else {
                "Mark as completed"
            },
            Icon::Check,
            move |id, _| Action::SetCompleted {
                id,
                completed: !completed,
            },
        ),
        separator().into(),
        super::task_dates::submenu(vm.clone(), ids.clone()),
    ];
    let move_items = destinations
        .into_iter()
        .map(|list| {
            super::task_action::move_to(vm.clone(), ids.clone(), list.id, list.name, selected)
        })
        .collect();
    if can_move {
        items.push(submenu(
            if count > 1 {
                "Move tasks to"
            } else {
                "Move task to"
            },
            Icon::MoveToList,
            move_items,
        ));
    }
    items.push(batch(
        vm.clone(),
        ids.clone(),
        if count > 1 {
            "Duplicate tasks"
        } else {
            "Duplicate task"
        },
        Icon::Copy,
        |id, _| Action::DuplicateTask(id),
    ));
    if can_move
        && count == 1
        && vm.sort() == SortOrder::Manual
        && matches!(vm.view(), View::List(_))
    {
        let up = vm.clone();
        let down = vm.clone();
        items.push(command("Move up", Icon::ArrowUp, move || {
            up.move_task_by(id, -1)
        }));
        items.push(command("Move down", Icon::ArrowDown, move || {
            down.move_task_by(id, 1)
        }));
    }
    if !selected {
        let select = vm.clone();
        items.push(command("Select task", Icon::CheckCircle, move || {
            select.toggle_task_selection(id)
        }));
    }
    if can_delete {
        items.push(separator().into());
        items.push(super::task_action::delete(vm, ids, selected));
    }
    items
}
