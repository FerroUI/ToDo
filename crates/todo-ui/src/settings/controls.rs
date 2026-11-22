use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ToggleButton;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_domain::{Action, Settings};
use todo_state::AppViewModel;

pub(super) fn section_label(label: &str, color: &str) -> Element {
    text_block()
        .text(label)
        .font_size(14.0)
        .font_weight(FontWeight::SemiBold)
        .foreground(color)
        .margin((0.0, 18.0, 0.0, 4.0))
        .into()
}

pub(super) fn toggle(
    vm: Rc<AppViewModel>,
    label: &str,
    checked: bool,
    update: fn(&mut Settings, bool),
) -> Element {
    toggle_switch()
        .child(
            text_block()
                .text(label)
                .font_size(14.0)
                .text_wrapping(TextWrapping::Wrap),
        )
        .automation_properties_name(label)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .is_checked(Some(checked))
        .font_size(14.0)
        .min_width(0.0)
        .min_height(48.0)
        // Native property changes cover pointer, keyboard and switch-knob dragging.
        .on_changed(ToggleButton::is_checked_property(), move |value| {
            let enabled = value.unwrap_or(false);
            if enabled != checked {
                let mut settings = vm.state().settings.clone();
                update(&mut settings, enabled);
                vm.dispatch(Action::SetSettings(settings));
            }
        })
        .into()
}
