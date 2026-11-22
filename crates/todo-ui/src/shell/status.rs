use crate::palette;
use ferroui_base::input::FocusManager;
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::TextWrapping;
use ferroui_controls::Control;
use ferroui_controls::automation::{AutomationLiveSetting, AutomationProperties};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{Icon, toolbar_button};
use todo_state::{AppViewModel, Panel};

pub(super) fn view(vm: Rc<AppViewModel>) -> Element {
    let p = palette(&vm);
    let can_undo = vm.status_can_undo();
    let mut content = grid()
        .column_definitions(if can_undo { "*,Auto,Auto" } else { "*,Auto" })
        .column_spacing(12.0)
        .child(
            text_block()
                .text(vm.status())
                .automation_properties_live_setting(AutomationLiveSetting::Polite)
                .font_size(14.0)
                .vertical_alignment(V::Center)
                .foreground(p.text)
                .text_wrapping(TextWrapping::Wrap),
        );
    if can_undo {
        let undo = vm.clone();
        content = content.child(
            button()
                .grid_column(1)
                .class("todo-action")
                .min_height(44.0)
                .automation_properties_name("Undo")
                .content("Undo")
                .on_click(move |event| activate(event, &undo, AppViewModel::undo)),
        );
    }
    content = content.child(
        toolbar_button(Icon::Close, "Dismiss message", p.secondary)
            .grid_column(if can_undo { 2 } else { 1 })
            .vertical_alignment(V::Center)
            .on_click(move |event| activate(event, &vm, AppViewModel::clear_status)),
    );
    border()
        .name("StatusMessage")
        .grid_row(2)
        .vertical_alignment(V::Bottom)
        .horizontal_alignment(H::Stretch)
        .margin((16.0, 0.0, 16.0, 12.0))
        .padding((14.0, 8.0))
        .corner_radius(5.0)
        .background(p.surface)
        .border_brush(p.border)
        .border_thickness(1.0)
        .max_width(650.0)
        .child(content)
        .into()
}

fn activate(event: &RoutedEventArgs, vm: &AppViewModel, action: fn(&AppViewModel)) {
    event.set_handled(true);
    // Both commands can remove their own button. Move focus before reconciliation;
    // a subsequent pane change still uses the shell's normal focus restoration.
    if let Some(root) = event
        .source()
        .and_then(|source| source.cast::<Control>())
        .and_then(|source| {
            source
                .get_visual_ancestors()
                .filter_map(|ancestor| ancestor.cast::<Control>())
                .find(|ancestor| ancestor.name().as_deref() == Some("TodoApp"))
        })
    {
        let names: &[&str] = match vm.panel() {
            Panel::Settings => &["AccountName", "Close settings", "NewTask"],
            Panel::Suggestions => &["Close suggestions", "NewTask"],
            _ if vm.selected().is_some() => &["TaskTitle", "NewStep", "Close details", "NewTask"],
            _ => &["NewTask"],
        };
        let focused = names.iter().any(|name| {
            root.find_descendant_of_type_where::<Control>(false, |control| {
                (control.name().as_deref() == Some(*name)
                    || AutomationProperties::get_name(control).as_deref() == Some(*name))
                    && FocusManager::can_focus(control)
            })
            .is_some_and(|control| control.focus())
        });
        if !focused {
            root.focus();
        }
    }
    action(vm);
}
