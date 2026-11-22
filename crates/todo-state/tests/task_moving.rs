mod common;

use common::{today, vm};
use todo_domain::{Action, SortOrder, View, TASKS_LIST};

#[test]
fn moving_to_a_list_appends_once_and_undo_restores_the_source_position() {
    let vm = vm();
    vm.add_task("Older source task".into());
    let source = vm.state().tasks[0].clone();
    vm.dispatch(Action::AddList {
        name: "Destination".into(),
    });
    let destination = vm.state().lists.last().unwrap().id;
    vm.add_task("Destination first".into());
    vm.add_task("Destination second".into());
    let ordered = |state: &todo_domain::AppState| {
        state
            .visible_tasks(&View::List(destination), SortOrder::Manual, true, today())
            .iter()
            .map(|task| task.id)
            .collect::<Vec<_>>()
    };
    let mut expected = ordered(&vm.state());
    expected.push(source.id);
    vm.dispatch(Action::MoveTask {
        id: source.id,
        list_id: destination,
    });
    assert_eq!(ordered(&vm.state()), expected);
    vm.undo();
    assert_eq!(vm.state().task(source.id), Some(&source));
    vm.redo();
    assert_eq!(ordered(&vm.state()), expected);

    // Moving to the current list is a no-op, including history and manual order.
    vm.dispatch(Action::MoveTask {
        id: source.id,
        list_id: destination,
    });
    vm.undo();
    assert_eq!(vm.state().task(source.id).unwrap().list_id, TASKS_LIST);
    assert_eq!(vm.state().task(source.id).unwrap().position, source.position);

    let mut overflow = vm.state().clone();
    overflow
        .tasks
        .iter_mut()
        .find(|task| task.list_id == destination)
        .unwrap()
        .position = i64::MAX;
    let unchanged = overflow.clone();
    assert!(overflow
        .apply(
            Action::MoveTask {
                id: source.id,
                list_id: destination,
            },
            today()
        )
        .is_err());
    assert_eq!(overflow, unchanged);
}
