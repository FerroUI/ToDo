use crate::palette;
use ferroui_base::input::{Key, KeyModifiers};
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::TextBox;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_state::AppViewModel;

pub(crate) fn edit_field(
    name: &str,
    value: String,
    placeholder: &str,
    multiline: bool,
    save: impl Fn(String) + 'static,
) -> El<TextBox> {
    let title = name == "TaskTitle";
    let notes = name == "TaskNotes";
    let required = title
        || name
            .strip_prefix("Step")
            .is_some_and(|id| id.parse::<u64>().is_ok());
    let enter_commits = !multiline || title;
    let original = value.clone();
    let commit: Rc<dyn Fn(&TextBox)> = Rc::new(move |field| {
        let text = field.text().unwrap_or_default();
        if required && text.trim().is_empty() {
            // Rejected blank names must not leave the retained editor visually blank.
            field.set_text(Some(original.as_str()));
        } else {
            save(text);
        }
    });
    let enter = commit.clone();
    text_box()
        .name(name)
        .class("todo-input")
        .text(value)
        .automation_properties_name(if notes { "Task notes" } else { placeholder })
        .automation_properties_help_text(if notes {
            "Enter adds a new line. Control+Enter saves the note."
        } else {
            "Press Enter to save."
        })
        .placeholder_text(placeholder)
        .font_size(if title { 18.0 } else { 14.0 })
        .font_weight(if title {
            FontWeight::SemiBold
        } else {
            FontWeight::Normal
        })
        .min_height(if notes { 96.0 } else { 36.0 })
        .vertical_content_alignment(VerticalAlignment::Top)
        .background("Transparent")
        .border_thickness(0.0)
        .padding((6.0, 8.0))
        .accepts_return(multiline && !title)
        .text_wrapping(TextWrapping::Wrap)
        .on_lost_focus(move |event| {
            if let Some(field) = event.source().and_then(|source| source.cast::<TextBox>()) {
                commit(&field);
            }
        })
        .on_key_down(move |event| {
            let modified_enter = event
                .key_modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::META);
            if event.key == Key::Return && (enter_commits || modified_enter) {
                if let Some(field) = event.source().and_then(|source| source.cast::<TextBox>()) {
                    enter(&field);
                }
                event.set_handled(true);
            }
        })
}

pub(super) fn section(vm: Rc<AppViewModel>, child: impl IntoChild) -> Element {
    border()
        .background(palette(&vm).surface)
        .corner_radius(4.0)
        .child(child)
        .into()
}
