mod focus;

use ferroui_base::input::Key;
use ferroui_base::media::FontWeight;
use ferroui_controls::TextBox;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_state::{AppViewModel, RenameTarget};

/// One transient editor for both list titles and group headings. Only a commit enters the model.
pub(crate) fn editor(
    vm: Rc<AppViewModel>,
    target: RenameTarget,
    size: f64,
    color: &str,
) -> Element {
    let draft = vm
        .rename_session()
        .filter(|edit| edit.target == target)
        .map(|edit| edit.draft)
        .unwrap_or_default();
    let change = vm.clone();
    let blur = vm.clone();
    let name = match target {
        RenameTarget::List(_) => "ListName",
        RenameTarget::Group(_) => "GroupName",
    };
    text_box()
        .key(format!("rename-{target:?}"))
        .name(name)
        .automation_properties_name(if matches!(target, RenameTarget::List(_)) {
            "List name"
        } else {
            "Group name"
        })
        .text(draft)
        .font_size(size)
        .font_weight(FontWeight::SemiBold)
        .foreground(color)
        .background("Transparent")
        .padding((4.0, 4.0))
        .min_width(0.0)
        .with(move |input| focus::activate(input, target))
        .on_text_changed(move |event| {
            if let Some(input) = event.source().and_then(|source| source.cast::<TextBox>()) {
                change.set_rename_draft(target, input.text().unwrap_or_default());
            }
        })
        .on_lost_focus(move |event| {
            if let Some(input) = event.source().and_then(|source| source.cast::<TextBox>()) {
                blur.set_rename_draft(target, input.text().unwrap_or_default());
            }
            blur.commit_rename(target);
        })
        .on_key_down(move |event| {
            if crate::editing::is_composing(event) {
                return;
            }
            match event.key {
                Key::Return => {
                    if let Some(input) = event.source().and_then(|source| source.cast::<TextBox>())
                    {
                        vm.set_rename_draft(target, input.text().unwrap_or_default());
                    }
                    vm.commit_rename(target);
                }
                Key::Escape => vm.cancel_rename(target),
                _ => return,
            }
            event.set_handled(true);
        })
        .into()
}
