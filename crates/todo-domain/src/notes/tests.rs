use super::*;

#[test]
fn native_note_features_survive_canonical_html_and_unicode() {
    let note = RichNote::from_html("<p><strong>日本語 👩🏽‍💻</strong> <em>é</em><br><u><s>done</s></u> <a href='https://example.com/a?x=1&amp;y=2'>site</a></p><ol start='3'><li>three</li><li>four</li></ol><ul><li>item</li></ul>").unwrap();
    assert_eq!(
        note.text(),
        "日本語 👩🏽‍💻 é\ndone site\n3. three\n4. four\n• item"
    );
    assert_eq!(RichNote::from_html(&note.to_html()).unwrap(), note);
    assert!(note.has_formatting());
}

#[test]
fn only_owned_markup_and_safe_links_leave_the_codec() {
    let note = RichNote::from_html("<script>alert(1)</script><p onclick='x()'><img src=x onerror=x()>safe <a href='&#106;avascript:alert(1)'>bad</a><a href='https://user:pass@example.com'>credentials</a><span style='font-weight:700;text-decoration:line-through;background:url(x)'>strong</span></p><svg><text>hidden</text></svg>").unwrap();
    assert_eq!(note.text(), "safe badcredentialsstrong");
    let html = note.to_html();
    assert!(!html.contains("script"));
    assert!(!html.contains("onclick"));
    assert!(!html.contains("href"));
    assert!(html.contains("<strong><s>strong</s></strong>"));
}

#[test]
fn whitespace_empty_lines_and_literals_round_trip() {
    let note = RichNote::plain("  leading\n\nA < B & C\ntrailing  \n");
    assert_eq!(
        RichNote::from_html(&note.to_html()).unwrap().text(),
        note.text()
    );
    assert!(RichNote::from_html(&"<span>".repeat(130)).is_err());
    assert!(RichNote::from_html(&"x".repeat(MAX_NOTE_BYTES + 1)).is_err());
}

#[test]
fn rich_action_is_atomic_and_plain_edits_retire_stale_formatting() {
    use crate::{Action, AppState, Date, TASKS_LIST};
    let day = Date::from_ymd_opt(2026, 11, 23).unwrap();
    let mut state = AppState::default();
    let id = state
        .apply(
            Action::AddTask {
                title: "Note".into(),
                list_id: TASKS_LIST,
            },
            day,
        )
        .unwrap()
        .created_task
        .unwrap();
    state
        .apply(
            Action::SetRichNotes {
                id,
                html: "<b>Hello #Team</b>".into(),
            },
            day,
        )
        .unwrap();
    let restored = AppState::from_json(&state.to_json().unwrap()).unwrap();
    assert_eq!(restored.task(id), state.task(id));
    assert!(state.task(id).unwrap().notes_html.is_some());
    state
        .apply(
            Action::SetNotes {
                id,
                notes: "Changed by a plain editor".into(),
            },
            day,
        )
        .unwrap();
    assert!(state.task(id).unwrap().notes_html.is_none());
    let before = state.clone();
    assert!(
        state
            .apply(
                Action::SetRichNotes {
                    id,
                    html: "x".repeat(MAX_NOTE_BYTES + 1)
                },
                day
            )
            .is_err()
    );
    assert_eq!(state, before);
}
