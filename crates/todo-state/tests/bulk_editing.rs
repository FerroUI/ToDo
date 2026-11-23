mod common;
use common::vm;
use todo_domain::{Action, ExternalTask};
use todo_state::Panel;

#[test]
fn custom_dates_reminders_and_category_edits_are_one_atomic_undo_each() {
    let vm = vm();
    for title in ["A", "B"] {
        vm.add_task(title.into());
    }
    let ids = vm.visible_task_ids();
    vm.show_panel(Panel::BulkReminder(ids.clone()));
    assert!(vm.save_tasks_date(&ids, vm.today(), Some("25:00")).is_err());
    assert!(vm.state().tasks.iter().all(|task| task.reminder.is_none()));
    vm.save_tasks_date(&ids, vm.today(), Some("09:15")).unwrap();
    assert_eq!(vm.panel(), Panel::None);
    assert!(vm.state().tasks.iter().all(|task| task
        .reminder
        .as_ref()
        .unwrap()
        .minutes_after_midnight
        == 555));
    vm.undo();
    assert!(vm.state().tasks.iter().all(|task| task.reminder.is_none()));
    vm.dispatch(Action::SetTags {
        id: ids[0],
        tags: vec!["Existing".into()],
    });
    vm.set_tasks_category(&ids, "Shared", true).unwrap();
    assert!(vm
        .state()
        .tasks
        .iter()
        .all(|task| task.tags.iter().any(|tag| tag == "Shared")));
    assert!(vm
        .state()
        .task(ids[0])
        .unwrap()
        .tags
        .contains(&"Existing".into()));
    vm.undo();
    assert_eq!(vm.state().task(ids[0]).unwrap().tags, vec!["Existing"]);
    assert!(vm.state().task(ids[1]).unwrap().tags.is_empty());
    vm.save_tasks_date(&ids, vm.today(), None).unwrap();
    assert!(vm
        .state()
        .tasks
        .iter()
        .all(|task| task.due_date == Some(vm.today())));
    vm.undo();
    assert!(vm.state().tasks.iter().all(|task| task.due_date.is_none()));
}

#[test]
fn missing_or_provider_owned_targets_reject_the_entire_bulk_edit() {
    let vm = vm();
    for title in ["Personal", "Planner"] {
        vm.add_task(title.into());
    }
    let mut state = vm.state().clone();
    let ids: Vec<_> = state.tasks.iter().map(|task| task.id).collect();
    state.tasks[1].external = Some(ExternalTask {
        provider: "Planner".into(),
        id: "remote".into(),
        url: "https://tasks.office.com/remote".into(),
    });
    vm.import_json(&state.to_json().unwrap()).unwrap();
    let before = vm.state().clone();
    assert!(vm.save_tasks_date(&ids, vm.today(), Some("09:00")).is_err());
    assert!(vm.set_tasks_category(&ids, "Category", true).is_err());
    assert!(vm
        .save_tasks_date(&[ids[0], 9999], vm.today(), None)
        .is_err());
    assert_eq!(*vm.state(), before);
}
