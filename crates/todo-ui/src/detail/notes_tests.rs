use super::*;
use std::cell::Cell;
use todo_domain::{notes::MAX_NOTE_BYTES, AppState, Date};

#[test]
fn rejected_rich_draft_survives_incoming_updates_until_an_edit_is_accepted() {
    let _scope = Dispatcher::unit_test_scope();
    let writes = Rc::new(Cell::new(0));
    let written = writes.clone();
    let vm = AppViewModel::new(
        AppState::new(),
        "2026-11-23".parse::<Date>().unwrap(),
        Rc::new(move |_| {
            written.set(written.get() + 1);
            Ok(())
        }),
    );
    vm.add_task("Draft rejection".into());
    let id = vm.state().tasks[0].id;
    vm.dispatch(Action::SetRichNotes {
        id,
        html: "<p><strong>Accepted note</strong></p>".into(),
    });
    let accepted = || {
        let state = vm.state();
        let task = state.task(id).unwrap();
        (task.notes.clone(), task.notes_html.clone())
    };
    let original = accepted();
    let baseline = RefCell::new(original.clone());
    let field = RichEditBox::new();

    // Escaping ampersands makes the serialized HTML exceed the domain limit
    // even though the native text is smaller. Use the real dispatch rejection.
    field.set_text(&"&".repeat(MAX_NOTE_BYTES / 5 + 1)).unwrap();
    field.select_all();
    field.toggle_bold().unwrap();
    let draft = field.rich_document().snapshot();
    let selection = field.editor().selection();
    assert!(document::read(&field).to_html().len() > MAX_NOTE_BYTES);
    let before_writes = writes.get();
    save_draft(&vm, id, &field, &baseline);
    assert!(vm.status().contains("1 MiB"));
    assert_eq!(accepted(), original);
    assert_eq!(writes.get(), before_writes);
    assert_eq!(*baseline.borrow(), original);

    vm.dispatch(Action::SetRichNotes {
        id,
        html: "<p><em>Incoming accepted note</em></p>".into(),
    });
    assert!(refresh_draft(&field, &baseline, &accepted()).is_none());
    assert_eq!(field.rich_document().snapshot(), draft);
    assert_eq!(field.editor().selection(), selection);
    assert!(field.can_undo());

    // A later valid local save becomes clean, so ordinary incoming updates can
    // once again refresh the retained editor and reset its obsolete history.
    field.set_text("Corrected 日本語 👩🏽‍💻").unwrap();
    field.select_all();
    field.toggle_bold().unwrap();
    let corrected = save_draft(&vm, id, &field, &baseline);
    assert_eq!(accepted(), (corrected.text(), canonical(&corrected)));
    assert_eq!(*baseline.borrow(), accepted());
    vm.dispatch(Action::SetRichNotes {
        id,
        html: "<p><u>A newer accepted note</u></p>".into(),
    });
    let refreshed = refresh_draft(&field, &baseline, &accepted()).unwrap();
    assert_eq!(document::read(&field), refreshed);
    assert_eq!((refreshed.text(), canonical(&refreshed)), accepted());
    assert!(!field.can_undo());
}
