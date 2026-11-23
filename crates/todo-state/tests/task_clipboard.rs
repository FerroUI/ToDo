mod common;
use common::vm;
use todo_domain::{Action, Attachment, View, TASKS_LIST};
use todo_state::{clipboard_task_lines, TaskClipboard};

fn fixture() -> (std::rc::Rc<todo_state::AppViewModel>, Vec<u64>, u64) {
    let vm = vm();
    for title in ["A <b>& 日本語", "B"] {
        vm.add_task(title.into());
    }
    let ids = vm.visible_task_ids();
    vm.dispatch(Action::AddStep {
        task_id: ids[0],
        title: "Nested step".into(),
    });
    vm.dispatch(Action::AddAttachment {
        task_id: ids[0],
        attachment: Attachment {
            id: 0,
            name: "local.txt".into(),
            mime_type: "text/plain".into(),
            size: 4,
            storage_key: "private-store-key".into(),
        },
    });
    vm.dispatch(Action::AddList {
        name: "Destination".into(),
    });
    let destination = vm.state().lists.last().unwrap().id;
    vm.navigate(View::List(TASKS_LIST));
    (vm, ids, destination)
}

#[test]
fn clipboard_round_trips_portable_metadata_and_escapes_html_without_exporting_local_files() {
    let (vm, ids, _) = fixture();
    let offer = vm.prepare_task_clipboard(&ids, false).unwrap();
    let json = offer.data.to_json().unwrap();
    assert!(!json.contains("private-store-key"));
    let html = offer.data.to_html().unwrap();
    assert!(html.contains("A &lt;b&gt;&amp; 日本語"));
    assert_eq!(TaskClipboard::from_json(&json).unwrap(), offer.data);
    assert_eq!(TaskClipboard::from_html(&html).unwrap(), Some(offer.data));
    assert!(TaskClipboard::from_html("<p>Ordinary HTML</p>")
        .unwrap()
        .is_none());
    assert!(TaskClipboard::from_html("<ul data-ferroui-todo-v1=\"xyz\">").is_err());
}

#[test]
fn copy_pastes_independent_tasks_in_order_with_single_undo_and_local_attachments() {
    let (vm, ids, destination) = fixture();
    let offer = vm.prepare_task_clipboard(&ids, false).unwrap();
    let payload = offer.data.clone();
    vm.accept_task_clipboard(offer).unwrap();
    let before = vm.state().clone();
    vm.paste_task_clipboard(
        vm.task_paste_target(View::List(destination)),
        Some(payload),
        "",
    )
    .unwrap();
    let state = vm.state();
    let copies: Vec<_> = state
        .tasks
        .iter()
        .filter(|task| task.list_id == destination)
        .collect();
    assert_eq!(copies.len(), 2);
    assert_eq!(copies[0].title, "A <b>& 日本語");
    assert_eq!(copies[0].attachments[0].storage_key, "private-store-key");
    assert_ne!(
        copies[0].attachments[0].id,
        state.task(ids[0]).unwrap().attachments[0].id
    );
    assert_eq!(copies[0].steps[0].title, "Nested step");
    drop(state);
    vm.undo();
    let mut restored = vm.state().clone();
    restored.next_id = before.next_id;
    assert_eq!(restored, before);
}

#[test]
fn cut_needs_a_successful_write_and_matches_every_source_before_moving() {
    let (vm, ids, destination) = fixture();
    let offer = vm.prepare_task_clipboard(&ids, true).unwrap();
    let payload = offer.data.clone();
    assert!(vm
        .paste_task_clipboard(
            vm.task_paste_target(View::List(destination)),
            Some(payload.clone()),
            ""
        )
        .is_err());
    assert!(vm
        .state()
        .tasks
        .iter()
        .all(|task| task.list_id == TASKS_LIST));
    vm.accept_task_clipboard(offer).unwrap();
    vm.dispatch(Action::RenameTask {
        id: ids[1],
        title: "Changed since cut".into(),
    });
    let before = vm.state().clone();
    assert!(vm
        .paste_task_clipboard(
            vm.task_paste_target(View::List(destination)),
            Some(payload),
            ""
        )
        .is_err());
    assert_eq!(*vm.state(), before);
}

#[test]
fn cut_is_consumed_once_and_an_undo_restores_the_whole_transfer() {
    let (vm, ids, destination) = fixture();
    let offer = vm.prepare_task_clipboard(&ids, true).unwrap();
    let payload = offer.data.clone();
    vm.accept_task_clipboard(offer).unwrap();
    let before = vm.state().clone();
    vm.paste_task_clipboard(
        vm.task_paste_target(View::List(destination)),
        Some(payload.clone()),
        "",
    )
    .unwrap();
    assert_eq!(vm.state().tasks.len(), 2);
    assert!(vm
        .state()
        .tasks
        .iter()
        .all(|task| task.list_id == destination));
    vm.undo();
    assert_eq!(*vm.state(), before);
    vm.paste_task_clipboard(
        vm.task_paste_target(View::List(destination)),
        Some(payload),
        "",
    )
    .unwrap();
    assert_eq!(
        vm.state().tasks.len(),
        4,
        "a second paste copies rather than repeating the cut"
    );
}

#[test]
fn cut_into_smart_views_applies_membership_with_the_transfer_in_one_undo() {
    for view in [
        View::MyDay,
        View::Important,
        View::Planned,
        View::AssignedToMe,
    ] {
        let (vm, ids, source) = fixture();
        vm.dispatch(Action::TransferTasks {
            ids: ids.clone(),
            list_id: source,
            before: None,
        });
        for id in &ids {
            vm.dispatch(Action::SetMyDay { id: *id, day: None });
        }
        let future = vm.today().succ_opt().unwrap();
        vm.dispatch(Action::SetStartDate {
            id: ids[0],
            date: Some(future),
        });
        let offer = vm.prepare_task_clipboard(&ids, true).unwrap();
        let payload = offer.data.clone();
        vm.accept_task_clipboard(offer).unwrap();
        let before = vm.state().clone();
        vm.paste_task_clipboard(vm.task_paste_target(view.clone()), Some(payload), "")
            .unwrap();
        for id in &ids {
            let state = vm.state();
            let task = state.task(*id).unwrap();
            assert_eq!(task.list_id, TASKS_LIST);
            match view {
                View::MyDay => assert_eq!(task.my_day, Some(vm.today())),
                View::Important => assert!(task.important),
                View::Planned => assert_eq!(
                    task.due_date,
                    Some(if *id == ids[0] { future } else { vm.today() })
                ),
                View::AssignedToMe => assert_eq!(task.assigned_to.as_deref(), Some("me")),
                _ => unreachable!(),
            }
        }
        vm.undo();
        assert_eq!(*vm.state(), before);
    }
}

#[test]
fn delayed_paste_and_document_replacement_cannot_retarget_old_cut_ids() {
    let (vm, ids, destination) = fixture();
    let offer = vm.prepare_task_clipboard(&ids, true).unwrap();
    let payload = offer.data.clone();
    vm.accept_task_clipboard(offer).unwrap();
    let target = vm.task_paste_target(View::List(destination));
    vm.navigate(View::Important);
    assert!(vm
        .paste_task_clipboard(target, Some(payload.clone()), "")
        .is_err());
    let json = vm.export_json().unwrap();
    vm.import_json(&json).unwrap();
    let before = vm.state().clone();
    assert!(vm
        .paste_task_clipboard(
            vm.task_paste_target(View::List(destination)),
            Some(payload),
            ""
        )
        .is_err());
    assert_eq!(*vm.state(), before);
}

#[test]
fn multiline_paste_handles_crlf_bullets_blank_lines_and_one_undo() {
    let vm = vm();
    let text = "\r\n - [ ] First\r\n• Second\n\n* Third\rFourth\n  -1 is a title\n";
    assert_eq!(
        clipboard_task_lines(text).unwrap(),
        ["First", "Second", "Third", "Fourth", "-1 is a title"]
    );
    vm.paste_task_clipboard(vm.task_paste_target(View::Important), None, text)
        .unwrap();
    assert_eq!(vm.state().tasks.len(), 5);
    assert!(vm.state().tasks.iter().all(|task| task.important));
    vm.undo();
    assert!(vm.state().tasks.is_empty());
    assert!(clipboard_task_lines(&"Task\n".repeat(1001)).is_err());
}
