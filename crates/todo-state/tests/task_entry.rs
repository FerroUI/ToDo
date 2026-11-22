mod common;

use common::{today, vm};
use todo_domain::{Action, TaskEntry, View};

#[test]
fn scheduled_creation_is_one_undo_step_and_direct_titles_remain_literal() {
    let vm = vm();
    vm.navigate(View::Important);
    vm.dispatch(Action::AddTaskEntry {
        entry: TaskEntry::parse_english("Call tomorrow at 3pm", today(), 600, true),
        view: vm.view(),
    });
    let task = vm.state().tasks[0].clone();
    assert_eq!(task.title, "Call");
    assert!(task.important);
    assert_eq!(
        task.reminder.as_ref().unwrap().date,
        today().succ_opt().unwrap()
    );
    vm.undo();
    assert!(vm.state().tasks.is_empty());
    vm.redo();
    assert_eq!(vm.state().tasks, vec![task]);

    vm.add_task("Keep tomorrow".into());
    let state = vm.state();
    let literal = state.tasks.last().unwrap();
    assert_eq!(literal.title, "Keep tomorrow");
    assert_eq!(literal.due_date, None);
    assert_eq!(literal.reminder, None);
}
