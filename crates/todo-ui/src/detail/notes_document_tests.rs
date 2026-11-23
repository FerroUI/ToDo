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
