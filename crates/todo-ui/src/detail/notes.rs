use super::notes_document as document;
use crate::palette;
use ferroui_base::input::{InputElement, Key, KeyModifiers};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::media::{FontStyle, FontWeight, TextDecorations, TextWrapping};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::{Control, RichEditBox, TextBox};
use ferroui_view::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use todo_domain::notes::{safe_link, RichNote};
use todo_domain::{Action, Id};
use todo_state::AppViewModel;

pub(super) fn view(vm: Rc<AppViewModel>, id: Id) -> Element {
    border()
        .key(format!("note-{id}"))
        .name(format!("NoteArea{id}"))
        .padding((6.0, 8.0))
        .child(rerender(move || editor(vm.clone(), id)))
        .into()
}

fn editor(vm: Rc<AppViewModel>, id: Id) -> Element {
    let Some(task) = vm.state().task(id).cloned() else {
        return Element::Empty;
    };
    let p = palette(&vm);
    let field = use_hook(NodeRef::<RichEditBox>::new);
    let address = use_hook(NodeRef::<TextBox>::new);
    let editing = use_signal(|| true);
    let link_open = use_signal(|| false);
    let error = use_signal(String::new);
    let revision = use_signal(|| 0u64);
    let baseline =
        use_hook(|| Rc::new(RefCell::new((task.notes.clone(), task.notes_html.clone()))));
    let preview = use_signal(|| document::from_task(&task));
    revision.get();
    let initial = document::from_task(&task);
    let original = (task.notes.clone(), task.notes_html.clone());
    let refresh_baseline = baseline.clone();
    use_effect_with(original.clone(), move |incoming| {
        if let Some(field) = field.get_untracked() {
            let previous = refresh_baseline.borrow().clone();
            let current = document::read(&field);
            let dirty = current.text() != previous.0 || canonical(&current) != previous.1;
            if !dirty && previous != *incoming && !field.editor().is_composing() {
                let note = incoming
                    .1
                    .as_deref()
                    .and_then(|s| RichNote::from_html(s).ok())
                    .unwrap_or_else(|| RichNote::plain(&incoming.0));
                if field
                    .rich_document()
                    .restore(&document::snapshot(&note))
                    .is_ok()
                {
                    field.editor().clear_history();
                    preview.set(note);
                }
            }
            *refresh_baseline.borrow_mut() = incoming.clone();
        }
    });
    let save: Rc<dyn Fn(&RichEditBox)> = {
        let vm = vm.clone();
        let baseline = baseline.clone();
        Rc::new(move |field| {
            if field.editor().is_composing() {
                return;
            }
            let note = document::read(field);
            *baseline.borrow_mut() = (note.text(), canonical(&note));
            preview.set(note.clone());
            commit(&vm, id, field);
        })
    };
    let mut toolbar = wrap_panel()
        .orientation(ferroui_base::layout::Orientation::Horizontal)
        .name("NoteFormatting")
        .is_visible(editing.get())
        .automation_properties_name("Note formatting");
    let native = field.get_untracked();
    for (name, label, text, checked, command) in [
        (
            "NoteBold",
            "Bold",
            "B",
            native.as_ref().is_some_and(|f| f.is_bold()),
            0,
        ),
        (
            "NoteItalic",
            "Italic",
            "I",
            native.as_ref().is_some_and(|f| f.is_italic()),
            1,
        ),
        (
            "NoteUnderline",
            "Underline",
            "U",
            native.as_ref().is_some_and(|f| f.is_underlined()),
            2,
        ),
        (
            "NoteStrike",
            "Strikethrough",
            "S",
            native.as_ref().is_some_and(|f| document::strike_state(f)),
            3,
        ),
        ("NoteBullets", "Bulleted list", "•", false, 4),
        ("NoteNumbered", "Numbered list", "1.", false, 5),
        ("NoteUndo", "Undo note edit", "↶", false, 6),
        ("NoteRedo", "Redo note edit", "↷", false, 7),
    ] {
        let mut text = text_block().text(text).font_size(15.0);
        if command == 0 {
            text = text.font_weight(FontWeight::Bold);
        }
        if command == 1 {
            text = text.font_style(FontStyle::Italic);
        }
        if command == 2 {
            text = text.text_decorations(TextDecorations::underline());
        }
        if command == 3 {
            text = text.text_decorations(TextDecorations::strikethrough());
        }
        toolbar = toolbar.child(
            toggle_button()
                .name(name)
                .class("todo-toolbar")
                .width(32.0)
                .height(32.0)
                .padding(4.0)
                .foreground(p.text)
                .is_checked(Some(checked))
                .automation_properties_name(label)
                .tool_tip_tip(label)
                .is_enabled(match command {
                    6 => native.as_ref().is_some_and(|f| f.can_undo()),
                    7 => native.as_ref().is_some_and(|f| f.can_redo()),
                    _ => true,
                })
                .child(text)
                .on_click(move |event| {
                    if let Some(field) = field.get_untracked() {
                        let result = match command {
                            0 => field.toggle_bold(),
                            1 => field.toggle_italic(),
                            2 => field.toggle_underline(),
                            3 => document::strike(&field),
                            4 => document::list(&field, false),
                            5 => document::list(&field, true),
                            6 => field.undo().map(|_| ()),
                            _ => field.redo().map(|_| ()),
                        };
                        error.set(
                            result
                                .err()
                                .map_or(String::new(), |error| error.to_string()),
                        );
                        revision.update(|n| *n = n.wrapping_add(1));
                        field.focus();
                    }
                    event.set_handled(true);
                }),
        );
    }
    toolbar = toolbar.child(
        button()
            .name("NoteLink")
            .class("todo-toolbar")
            .content("Link")
            .height(32.0)
            .padding((8.0, 3.0))
            .automation_properties_name("Insert or edit link")
            .on_click(move |event| {
                link_open.set(true);
                error.set(String::new());
                if let Some(address) = address.get_untracked() {
                    let current = field
                        .get_untracked()
                        .and_then(|field| document::selected_link(&field))
                        .unwrap_or_default();
                    address.set_text(Some(&current));
                }
                Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(field) = address.get_untracked() {
                            field.focus();
                            field.select_all();
                        }
                    },
                    DispatcherPriority::BACKGROUND,
                );
                event.set_handled(true);
            }),
    );
    let saved = save.clone();
    let mounted_save = save.clone();
    let native = rich_edit_box().name("TaskNotes").node_ref(field).is_visible(editing.get())
        .class("todo-input").placeholder_text("Add a note").min_height(110.0).font_size(14.0)
        .foreground(p.text).background("Transparent").border_thickness(0.0).padding((6.0, 8.0))
        .accepts_return(true).accepts_tab(false).text_wrapping(TextWrapping::Wrap)
        .automation_properties_name("Task notes")
        .automation_properties_help_text("Rich-text note. Enter adds a line. Control or Command+B, I and U format text. Control or Command+Enter saves. Use Preview note to follow links and hashtags.")
        .with(move |field| {
            if let Err(problem) = field.rich_document().restore(&document::snapshot(&initial)) { error.set(problem.to_string()); }
            field.editor().clear_history();
            let changed = field.text_changed(move |_, _| revision.update(|n| *n = n.wrapping_add(1)));
            let selected = field.selection_changed(move |_| revision.update(|n| *n = n.wrapping_add(1)));
            let weak = field.downgrade(); let save = mounted_save.clone();
            let token = field.add_handler_with(InputElement::key_down_event(), move |_, event| {
                let Some(field) = weak.upgrade() else { return };
                if field.editor().is_composing() { return; }
                if event.key == Key::Return {
                    if event.key_modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::META) {
                        event.set_handled(true); save(&field);
                    } else if event.key_modifiers.is_empty() {
                        match document::list_enter(&field) {
                            Ok(true) => event.set_handled(true),
                            Err(problem) => { error.set(problem.to_string()); event.set_handled(true); }
                            _ => {}
                        }
                    }
                }
            }, RoutingStrategies::TUNNEL, false);
            let weak = field.downgrade();
            on_cleanup(move || { drop((changed, selected)); if let Some(field) = weak.upgrade() { field.remove_handler(InputElement::key_down_event(), token); } });
        })
        .on_lost_focus(move |event| {
            let inside = event.new_focused_element.as_ref().is_some_and(|next| next.get_self_and_visual_ancestors()
                .filter_map(|v| v.cast::<Control>()).any(|v| v.name().as_deref() == Some(&format!("NoteArea{id}"))));
            if !inside { if let Some(field) = field.get_untracked() { saved(&field); } }
        });
    let preview_note = preview.get();
    let preview_vm = vm.clone();
    let toggle = save.clone();
    let mut panel = stack_panel().spacing(4.0).child(toolbar)
        .child(stack_panel().is_visible(link_open.get() && editing.get()).spacing(4.0)
            .child(text_box().name("NoteLinkAddress").node_ref(address).placeholder_text("https://example.com")
                .automation_properties_name("Link address").font_size(14.0).min_width(0.0))
            .child(wrap_panel().orientation(ferroui_base::layout::Orientation::Horizontal)
                .child(button().name("NoteApplyLink").content("Apply link").on_click(move |event| {
                    let uri = address.get_untracked().and_then(|f| f.text()).unwrap_or_default();
                    if let Some(uri) = safe_link(&uri) {
                        if let Some(field) = field.get_untracked() {
                            match document::link(&field, Some(&uri)) {
                                Ok(()) => { link_open.set(false); error.set(String::new()); field.focus(); }
                                Err(problem) => error.set(problem.to_string()),
                            }
                        }
                    } else { error.set("Enter a complete http:// or https:// address without credentials.".into()); }
                    event.set_handled(true);
                }))
                .child(button().name("NoteRemoveLink").content("Remove link").on_click(move |event| {
                    if let Some(field) = field.get_untracked() {
                        match document::link(&field, None) { Ok(()) => { link_open.set(false); field.focus(); }, Err(problem) => error.set(problem.to_string()) }
                    }
                    event.set_handled(true);
                }))
                .child(button().content("Cancel").automation_properties_name("Cancel link editing").on_click(move |event| {
                    link_open.set(false); error.set(String::new()); if let Some(field) = field.get_untracked() { field.focus(); } event.set_handled(true);
                }))))
        .child(native)
        .child(border().is_visible(!editing.get()).child(super::note_links::view(preview_vm, preview_note, "NotePreview", p.text, p.accent)))
        .child(button().name("NotePreviewToggle").class("todo-toolbar").padding((8.0, 5.0))
            .content(if editing.get() { "Preview note" } else { "Edit note" })
            .automation_properties_name(if editing.get() { "Preview note" } else { "Edit note" })
            .on_click(move |event| {
                if let Some(field) = field.get_untracked() {
                    if editing.get_untracked() { toggle(&field); }
                    editing.update(|value| *value = !*value);
                    if editing.get_untracked() { field.focus(); }
                }
                event.set_handled(true);
            }));
    if !error.get().is_empty() {
        panel = panel.child(
            text_block()
                .name("NoteValidation")
                .text(error.get())
                .foreground(p.secondary)
                .text_wrapping(TextWrapping::Wrap),
        );
    }
    panel.into()
}

fn canonical(note: &RichNote) -> Option<String> {
    note.has_formatting().then(|| note.to_html())
}

pub(crate) fn commit(vm: &AppViewModel, id: Id, field: &RichEditBox) {
    let note = document::read(field);
    let state = vm.state();
    let changed = state
        .task(id)
        .is_some_and(|task| task.notes != note.text() || task.notes_html != canonical(&note));
    drop(state);
    if changed {
        vm.dispatch(Action::SetRichNotes {
            id,
            html: note.to_html(),
        });
    }
}
