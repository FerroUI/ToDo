use super::*;
use ferroui_base::threading::Dispatcher;

#[test]
fn native_lists_keep_unicode_selection_and_undo() {
    let _scope = Dispatcher::unit_test_scope();
    let field = RichEditBox::new();
    field.set_text("日本語 👩🏽‍💻\nSecond").unwrap();
    field.editor().clear_history();
    field.select_all();
    let before = field.rich_document().snapshot();
    list(&field, false).unwrap();
    assert_eq!(read(&field).text(), "• 日本語 👩🏽‍💻\n• Second");
    assert!(read(&field).to_html().contains("<ul>"));
    field.undo().unwrap();
    assert_eq!(field.rich_document().snapshot(), before);
    field.redo().unwrap();
    list(&field, true).unwrap();
    assert_eq!(read(&field).text(), "1. 日本語 👩🏽‍💻\n2. Second");
    let end = field.text().encode_utf16().count();
    field.select(end, end);
    assert!(list_enter(&field).unwrap());
    assert_eq!(read(&field).text(), "1. 日本語 👩🏽‍💻\n2. Second\n3. ");
    assert!(list_enter(&field).unwrap());
    assert_eq!(read(&field).text(), "1. 日本語 👩🏽‍💻\n2. Second\n");
}

#[test]
fn native_formatting_links_and_ime_roundtrip_without_flattening() {
    let _scope = Dispatcher::unit_test_scope();
    let field = RichEditBox::new();
    let note = RichNote::from_html("<p><strong>Hello</strong> <u>é</u></p>").unwrap();
    field.rich_document().restore(&snapshot(&note)).unwrap();
    field.editor().clear_history();
    assert_eq!(read(&field), note);
    field.select(6, 8);
    strike(&field).unwrap();
    assert!(read(&field).to_html().contains("<u><s>é</s></u>"));
    link(&field, Some("https://example.com/")).unwrap();
    assert_eq!(
        selected_link(&field).as_deref(),
        Some("https://example.com/")
    );
    let before = field.rich_document().snapshot();
    field.set_preedit_text(Some("日本語"), Some(3)).unwrap();
    assert!(!list_enter(&field).unwrap());
    field.cancel_composition().unwrap();
    assert_eq!(field.rich_document().snapshot(), before);
    field.set_preedit_text(Some("日本"), Some(2)).unwrap();
    field.commit_composition("日本語").unwrap();
    assert_eq!(read(&field).text(), "Hello 日本語");
    field.undo().unwrap();
    assert_eq!(field.rich_document().snapshot(), before);
    field.select(7, 7);
    assert_eq!(
        selected_link(&field).as_deref(),
        Some("https://example.com/")
    );
    link(&field, Some("https://example.org/edited")).unwrap();
    assert_eq!(field.editor().selection(), EditorSelection::caret(7));
    assert_eq!(
        selected_link(&field).as_deref(),
        Some("https://example.org/edited")
    );
    link(&field, None).unwrap();
    assert_eq!(selected_link(&field), None);
    assert_eq!(read(&field).text(), "Hello é");
}

#[test]
fn lifecycle_flush_commits_hidden_rich_note_and_composition_to_its_own_task() {
    use ferroui_controls::Border;
    use std::{cell::RefCell, rc::Rc};
    use todo_domain::{AppState, Date};
    use todo_state::AppViewModel;

    let _scope = Dispatcher::unit_test_scope();
    let saved = Rc::new(RefCell::new(String::new()));
    let durable = saved.clone();
    let vm = AppViewModel::new(
        AppState::new(),
        "2026-11-23".parse::<Date>().unwrap(),
        Rc::new(move |json| {
            *durable.borrow_mut() = json.to_owned();
            Ok(())
        }),
    );
    vm.add_task("First task".into());
    let first = vm.state().tasks[0].id;
    vm.add_task("Second task".into());
    let second = vm.state().tasks[1].id;
    vm.select(Some(first));
    let area = Border::new();
    area.set_name(Some(format!("NoteArea{first}")));
    let field = RichEditBox::new();
    field.set_name(Some("TaskNotes".into()));
    area.set_child(field.clone());
    let note = RichNote::from_html("<p><strong>日本語 👩🏽‍💻</strong></p>").unwrap();
    field.rich_document().restore(&snapshot(&note)).unwrap();
    let end = field.text().encode_utf16().count();
    field.select(end, end);
    field.set_preedit_text(Some("候補"), Some(2)).unwrap();
    field.set_is_visible(false); // Preview retains the authoritative editor.
    crate::editing::commit_pending_edits(&vm, &area);
    assert!(!field.editor().is_composing());
    let restored = AppState::from_json(&saved.borrow()).unwrap();
    assert_eq!(restored.task(first).unwrap().notes, "日本語 👩🏽‍💻候補");
    assert!(restored
        .task(first)
        .unwrap()
        .notes_html
        .as_deref()
        .unwrap()
        .contains("<strong>"));
    assert_eq!(read(&field).text(), restored.task(first).unwrap().notes);
    let once = saved.borrow().clone();
    crate::editing::commit_pending_edits(&vm, &area);
    assert_eq!(
        *saved.borrow(),
        once,
        "a repeated lifecycle flush cannot duplicate composition"
    );

    // The VM has moved to the next task but the old retained pane has not yet
    // reconciled. A host flush at that point cannot overwrite the new task.
    vm.select(Some(second));
    field.set_text("An old retained draft").unwrap();
    crate::editing::commit_pending_edits(&vm, &area);
    assert!(vm.state().task(second).unwrap().notes.is_empty());
    assert_eq!(vm.state().task(first).unwrap().notes, "日本語 👩🏽‍💻候補");
}
