use super::item::{command, destructive};
use ferroui_view::Element;
use std::{collections::BTreeSet, rc::Rc};
use todo_controls::Icon;
use todo_domain::{Action, Id};
use todo_state::AppViewModel;

pub(super) fn batch(
    vm: Rc<AppViewModel>,
    ids: Rc<[Id]>,
    label: impl Into<String>,
    symbol: Icon,
    action: impl Fn(Id, &AppViewModel) -> Action + 'static,
) -> Element {
    command(label, symbol, move || {
        let actions = existing_actions(&vm, &ids, &action);
        if !actions.is_empty() {
            vm.dispatch(Action::Batch(actions));
        }
    })
}

pub(super) fn move_to(
    vm: Rc<AppViewModel>,
    ids: Rc<[Id]>,
    list_id: Id,
    label: String,
    selected: bool,
) -> Element {
    command(label, Icon::List, move || {
        vm.transfer_tasks(ids.to_vec(), list_id, None);
        let moved = ids.iter().all(|id| {
            vm.state()
                .task(*id)
                .is_some_and(|task| task.list_id == list_id)
        });
        if selected && moved {
            vm.end_selection();
        }
    })
}

pub(super) fn delete(vm: Rc<AppViewModel>, ids: Rc<[Id]>, selected: bool) -> Element {
    let label = if ids.len() > 1 {
        format!("Delete {} tasks", ids.len())
    } else {
        "Delete task".into()
    };
    destructive(label, Icon::Trash, vm.is_dark(), move || {
        if selected {
            // Confirmation uses the selection. Bind it to the opening snapshot,
            // even if an asynchronous refresh changed selection while open.
            let intended: BTreeSet<_> = ids.iter().copied().collect();
            if vm.selection() != intended {
                vm.select_all_tasks(&ids);
            }
            vm.delete_selection();
        } else if let Some(id) = ids.first() {
            vm.delete_task(*id);
        }
    })
}

fn existing_actions(
    vm: &AppViewModel,
    ids: &[Id],
    action: &impl Fn(Id, &AppViewModel) -> Action,
) -> Vec<Action> {
    let state = vm.state();
    ids.iter()
        .copied()
        .filter(|id| state.task(*id).is_some())
        .map(|id| action(id, vm))
        .collect()
}
