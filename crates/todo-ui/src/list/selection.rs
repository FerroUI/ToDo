use crate::{palette, rows::Rows};
use ferroui_base::layout::{Orientation, VerticalAlignment as V};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{icon_button, Icon};
use todo_state::{AppViewModel, Panel};

pub(super) fn view(vm: Rc<AppViewModel>, rows: Rc<Rows>) -> Element {
    let p = palette(&vm);
    let count = vm.selection_count();
    let all = vm.clone();
    let complete = vm.clone();
    let move_tasks = vm.clone();
    let delete = vm.clone();
    let close = vm.clone();
    border()
        .corner_radius(5.0)
        .background(p.surface)
        .min_height(54.0)
        .padding((8.0, 5.0))
        .child(
            grid()
                .column_definitions("Auto,*,Auto")
                .child(icon_button(
                    Icon::CheckCircle,
                    "Select all visible tasks",
                    move || all.select_all_tasks(&rows.task_ids()),
                ))
                .child(
                    text_block()
                        .grid_column(1)
                        .text(format!("{count} selected"))
                        .font_size(13.0)
                        .foreground(p.text)
                        .vertical_alignment(V::Center)
                        .margin((6.0, 0.0)),
                )
                .child(
                    stack_panel()
                        .grid_column(2)
                        .orientation(Orientation::Horizontal)
                        .spacing(1.0)
                        .child(border().is_enabled(count > 0).child(icon_button(
                            Icon::Check,
                            "Complete selected tasks",
                            move || complete.complete_selected_tasks(),
                        )))
                        .child(border().is_enabled(count > 0).child(icon_button(
                            Icon::List,
                            "Move selected tasks",
                            move || move_tasks.show_panel(Panel::MoveSelection),
                        )))
                        .child(border().is_enabled(count > 0).child(icon_button(
                            Icon::Trash,
                            "Delete selected tasks",
                            move || delete.delete_selection(),
                        )))
                        .child(icon_button(Icon::Close, "Exit task selection", move || {
                            close.end_selection()
                        })),
                ),
        )
        .into()
}
