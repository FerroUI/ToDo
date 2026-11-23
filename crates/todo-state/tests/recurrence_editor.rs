mod common;
use common::*;
use std::rc::Rc;
use todo_domain::{Action, AppState, Date, Recurrence, RepeatDay, RepeatEnd, RepeatUnit};
use todo_state::{AppViewModel, Panel, RepeatDraft};
fn date(value: &str) -> Date {
    value.parse().unwrap()
}

#[test]
fn full_repeat_draft_validates_before_mutating_and_saves_one_atomic_undo() {
    let vm = vm();
    vm.add_task("Schedule".into());
    let id = vm.state().tasks[0].id;
    vm.show_panel(Panel::Repeat(id));
    let before = vm.export_json().unwrap();
    let mut draft = RepeatDraft::from_task(vm.state().task(id), today(), true);
    draft.unit = RepeatUnit::Week;
    draft.days.clear();
    assert_eq!(
        vm.save_repeat_draft(id, &draft).unwrap_err().field,
        "RepeatDays"
    );
    draft.days = vec![RepeatDay::Monday, RepeatDay::Thursday];
    draft.start = "2026-11-23".into();
    draft.ending = 1;
    draft.until = "2026-11-22".into();
    assert_eq!(
        vm.save_repeat_draft(id, &draft).unwrap_err().field,
        "RepeatUntil"
    );
    assert_eq!(vm.export_json().unwrap(), before);
    assert_eq!(vm.panel(), Panel::Repeat(id));
    draft.ending = 2;
    draft.count = "4".into();
    draft.time_zone = " Europe/Warsaw ".into();
    vm.save_repeat_draft(id, &draft).unwrap();
    let saved = vm.export_json().unwrap();
    let rule = vm.state().task(id).unwrap().recurrence.clone().unwrap();
    assert_eq!(rule.end, RepeatEnd::After(4));
    assert_eq!(rule.time_zone.as_deref(), Some("Europe/Warsaw"));
    assert_eq!(
        vm.state().task(id).unwrap().due_date,
        Some(date("2026-11-23"))
    );
    assert_eq!(
        AppState::from_json(&saved)
            .unwrap()
            .task(id)
            .unwrap()
            .recurrence,
        Some(rule)
    );
    vm.undo();
    assert_eq!(vm.export_json().unwrap(), before);
    vm.redo();
    assert_eq!(vm.export_json().unwrap(), saved);
}

#[test]
fn reopening_progressed_month_end_does_not_rewind_or_reanchor_its_series() {
    let today = date("2027-01-31");
    let vm = AppViewModel::new(AppState::new(), today, Rc::new(|_| Ok(())));
    vm.add_task("Month end".into());
    let id = vm.state().tasks[0].id;
    vm.dispatch(Action::SetDueDate {
        id,
        date: Some(today),
    });
    vm.dispatch(Action::SetRecurrence {
        id,
        recurrence: Some(Recurrence {
            end: RepeatEnd::After(4),
            ..Recurrence::monthly()
        }),
    });
    vm.dispatch(Action::SetCompleted {
        id,
        completed: true,
    });
    let successor = vm
        .state()
        .tasks
        .iter()
        .find(|task| !task.completed)
        .unwrap()
        .id;
    let before = vm.export_json().unwrap();
    let draft = RepeatDraft::from_task(vm.state().task(successor), today, false);
    vm.save_repeat_draft(successor, &draft).unwrap();
    assert_eq!(vm.export_json().unwrap(), before);
    assert_eq!(
        vm.state().task(successor).unwrap().due_date,
        Some(date("2027-02-28"))
    );
    vm.dispatch(Action::SetCompleted {
        id: successor,
        completed: true,
    });
    assert_eq!(
        vm.state()
            .tasks
            .iter()
            .find(|task| !task.completed)
            .unwrap()
            .due_date,
        Some(date("2027-03-31"))
    );
}

#[test]
fn bulk_repeat_validates_every_target_then_batches_the_whole_selection() {
    let vm = vm();
    vm.add_task("First".into());
    vm.add_task("Second".into());
    let ids: Vec<_> = vm.state().tasks.iter().map(|task| task.id).collect();
    let mut draft = RepeatDraft::from_task(None, today(), false);
    draft.unit = RepeatUnit::Month;
    draft.day = "1".into();
    draft.start = "2026-11-21".into();
    let before = vm.export_json().unwrap();
    assert!(vm
        .save_bulk_repeat_draft(&[ids[0], u64::MAX], &draft)
        .is_err());
    assert_eq!(vm.export_json().unwrap(), before);
    vm.save_bulk_repeat_draft(&[ids[0], ids[1], ids[0]], &draft)
        .unwrap();
    assert!(vm
        .state()
        .tasks
        .iter()
        .all(|task| task.due_date == Some(date("2026-12-01"))
            && task.recurrence.as_ref().unwrap().anchor == Some(date("2026-11-21"))));
    vm.undo();
    assert_eq!(vm.export_json().unwrap(), before);
}

#[test]
fn impossible_ranges_and_bad_counts_have_field_specific_validation() {
    let mut draft = RepeatDraft::from_task(None, today(), false);
    draft.unit = RepeatUnit::Week;
    draft.days = vec![RepeatDay::Monday];
    draft.start = "2026-11-21".into();
    draft.ending = 1;
    draft.until = "2026-11-22".into();
    assert_eq!(draft.rule().unwrap_err().field, "RepeatUntil");
    draft.ending = 2;
    for value in ["0", "2147483648", "one", "-2"] {
        draft.count = value.into();
        assert_eq!(draft.rule().unwrap_err().field, "RepeatCount");
    }
    draft.count = "2".into();
    assert!(draft.preview(today()).contains("Mon, 23 Nov 2026"));
    assert!(draft.preview(date("2027-01-01")).contains("no remaining"));
}
