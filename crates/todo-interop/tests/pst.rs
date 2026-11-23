use todo_domain::{AppState, Date};
use todo_interop::import_pst;

const PST: &[u8] = include_bytes!("fixtures/tasks.pst");
fn today() -> Date {
    Date::from_ymd_opt(2026, 11, 22).unwrap()
}

#[test]
fn rich_notes_and_outlook_formatting_edits_survive_pst() {
    let report = import_pst(
        include_bytes!("fixtures/rich-notes.pst"),
        &AppState::new(),
        today(),
    )
    .unwrap();
    let restored = report
        .archive
        .document
        .tasks
        .iter()
        .find(|task| task.title == "Rich note roundtrip")
        .unwrap();
    assert_eq!(restored.notes, "日本語 #Release link\n• Review 👩🏽‍💻");
    assert!(restored
        .notes_html
        .as_ref()
        .unwrap()
        .contains("<strong>日本語 #Release</strong>"));
    assert!(restored.notes_html.as_ref().unwrap().contains("<ul>"));
    assert_eq!(restored.steps[0].title, "Check");
    let edited = report
        .archive
        .document
        .tasks
        .iter()
        .find(|task| task.title == "Outlook formatting edit")
        .unwrap();
    assert!(edited
        .notes_html
        .as_ref()
        .unwrap()
        .contains("<em>日本語 #Release</em>"));
    assert!(!edited.notes_html.as_ref().unwrap().contains("<strong>"));
    assert!(edited.notes.contains("Steps:\n[ ] Check"));
    assert_eq!(
        AppState::from_json(&report.archive.document.to_json().unwrap())
            .unwrap()
            .tasks,
        report.archive.document.tasks
    );
}

#[test]
fn reads_real_unicode_task_store_named_properties_and_file_bytes() {
    let report = import_pst(PST, &AppState::new(), today()).unwrap();
    assert_eq!(report.archive.document.tasks.len(), 2);
    assert_eq!(report.skipped_messages, 1);
    let task = &report.archive.document.tasks[0];
    assert_eq!(task.title, "Review design — Zażółć");
    assert_eq!(task.notes, "A plain note\nSecond line.");
    assert_eq!(task.list_id, todo_domain::TASKS_LIST);
    assert!(task.important && task.completed);
    assert_eq!(task.due_date.unwrap().to_string(), "2026-11-19");
    assert_eq!(task.completed_on.unwrap().to_string(), "2026-11-20");
    assert_eq!(task.tags, vec!["Project, A", "Żółty"]);
    assert_eq!(task.steps.len(), 1);
    assert!(task.steps[0].completed);
    assert_eq!(task.steps[0].title, "Review section");
    assert_eq!(task.my_day.unwrap().to_string(), "2026-11-22");
    assert_eq!(task.recurrence.as_ref().unwrap().every, 2);
    assert_eq!(task.attachments[0].name, "notes.txt");
    assert_eq!(
        task.microsoft_data.as_ref().unwrap()["pst"]["properties"]["0e1b"]["value"],
        true
    );
    assert_eq!(report.archive.files[0].base64, "ZmlsZSBieXRlcyDinJM=");
    use base64::Engine;
    let large = base64::engine::general_purpose::STANDARD
        .decode(&report.archive.files[1].base64)
        .unwrap();
    assert_eq!(large.len(), 9000);
    assert!(large
        .iter()
        .enumerate()
        .all(|(index, value)| *value == (index % 251) as u8));
    assert_eq!(report.utc_reminders[0].date_time, "2026-11-18T10:15:00Z");
    assert!(!report.archive.document.settings.auto_sync);
    assert!(task.external.is_none() && !task.flagged_email);
    let standard = &report.archive.document.tasks[1];
    assert_eq!(standard.title, "Plain Outlook task");
    assert!(!standard.completed && !standard.important);
    assert!(standard.steps.is_empty() && standard.reminder.is_none());
    assert_eq!(
        standard.microsoft_data.as_ref().unwrap()["pst"]["properties"]["0e1b"]["value"],
        false
    );
    assert!(report
        .archive
        .document
        .lists
        .iter()
        .any(|list| list.name == "Empty task list"));
    report.archive.document.validate().unwrap();
}

#[test]
fn malformed_truncated_pst_does_not_modify_the_source_document() {
    let current = AppState::new();
    let original = current.clone();
    assert!(import_pst(&PST[..PST.len() / 2], &current, today()).is_err());
    assert_eq!(current, original);
}

#[test]
fn malformed_btree_entry_counts_are_rejected_before_traversal() {
    // MS-PST Unicode ROOT.brefNBT.ib is at byte 224; a BTPAGE stores
    // cEnt/cEntMax at bytes 488/489. Corrupt counts are rejected before CRC
    // or record allocation, so this fixture cannot trigger a large allocation.
    let mut bytes = PST.to_vec();
    let root = u64::from_le_bytes(bytes[224..232].try_into().unwrap()) as usize;
    bytes[root + 488] = 255;
    bytes[root + 489] = 1;
    let error = import_pst(&bytes, &AppState::new(), today()).err().unwrap();
    assert!(
        error.to_string().contains("Invalid BTPAGE cEnt: 255"),
        "{error}"
    );
}

#[test]
fn source_status_importance_start_and_precise_times_survive_export() {
    let report = import_pst(
        include_bytes!("fixtures/fidelity.pst"),
        &AppState::new(),
        today(),
    )
    .unwrap();
    let tasks = &report.archive.document.tasks;
    assert_eq!(tasks.len(), 5);
    for (task, status) in tasks.iter().zip([
        "inProgress",
        "waitingOnOthers",
        "deferred",
        "completed",
        "notStarted",
    ]) {
        let data = task.microsoft_data.as_ref().unwrap();
        assert_eq!(data["status"], status);
        assert_eq!(
            data["importance"],
            if task.title == "editedCompleted" {
                "high"
            } else if task.title == "editedNormal" {
                "normal"
            } else {
                "low"
            }
        );
        assert_eq!(data["startDateTime"]["dateTime"], "2026-10-28T00:00:00");
        assert_eq!(data["createdDateTime"], "2026-10-01T12:30:45.123Z");
        assert_eq!(data["lastModifiedDateTime"], "2026-11-01T13:20:10Z");
        assert_eq!(data["bodyLastModifiedDateTime"], "2026-11-01T13:00:00Z");
        let named = data["pst"]["namedProperties"].as_object().unwrap();
        let percent = named
            .iter()
            .find(|(_, value)| {
                value["guid"] == "00062003-0000-0000-c000-000000000046"
                    && value["name"] == "00008102"
            })
            .unwrap()
            .0;
        let expected = if task.title == "editedCompleted" {
            1.0
        } else if task.title == "editedNormal" {
            0.0
        } else {
            0.375
        };
        assert_eq!(
            data["pst"]["properties"][percent]["value"].as_f64(),
            Some(expected)
        );
    }
}
