mod background;
mod composer;
mod empty;
mod entry_hint;
mod group;
mod header;
mod interactions;
mod keyboard;
mod selection;
mod sort_status;
mod task_row;

pub(crate) use background::list_accent;
pub use task_row::row;

use crate::rows::Rows;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_domain::View;
use todo_state::AppViewModel;

pub fn view(vm: Rc<AppViewModel>, rows: Rc<Rows>, drafts: Rc<crate::shell::Drafts>) -> Element {
    let accent = list_accent(&vm);
    let my_day = matches!(vm.view(), View::MyDay);
    let dark = vm.is_dark();
    let background = background::list_background(&vm, my_day);
    let foreground = if my_day { "#FFFFFF" } else { accent.as_str() };
    let inset = if vm.is_narrow() { 16.0 } else { 32.0 };
    let header = header::view(vm.clone(), foreground, inset, drafts.clone());
    let is_empty = rows.is_empty();
    let key_vm = vm.clone();
    let key_rows = rows.clone();
    let mut center = grid().grid_row(1).margin((inset, 0.0, inset, 12.0));
    center = center.child(
        list_box()
            .name("TaskList")
            .node_ref(drafts.task_list)
            .class("todo-list")
            .automation_properties_name("Tasks")
            .background("Transparent")
            .border_thickness(0.0)
            .padding(0.0)
            .items_source(rows.source())
            .item_template(rows.template.clone())
            .with(move |list| keyboard::attach(list, key_vm.clone(), key_rows.clone()))
            .scroll_viewer_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled),
    );
    if is_empty {
        center = center.child(empty::view(vm.clone(), foreground));
    }
    let mut layout = grid()
        .row_definitions("Auto,*,Auto")
        .background(background.as_str())
        .child(header)
        .child(center)
        .child(
            border()
                .grid_row(2)
                .margin((inset, 8.0, inset, 24.0))
                .child(if vm.selection_mode() {
                    selection::view(vm.clone(), rows)
                } else {
                    composer::view(vm.clone(), my_day, drafts)
                }),
        );
    // The My Day surface uses an original, quiet gradient-like landscape drawn by FerroUI.
    if my_day {
        layout = grid()
            .background(background.as_str())
            .child(background::landscape(dark, &background))
            .child(layout.background("Transparent"));
    }
    layout.into()
}
