use crate::rows::Row;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{star_button, task_check};
use todo_domain::Action;
use todo_state::{AppViewModel, Panel};
pub fn row(vm: Rc<AppViewModel>, row: Row) -> Element {
    match row {
        Row::Header {
            label,
            completed,
            expanded,
            foreground,
            ..
        } => super::group::view(vm, label, completed, expanded, foreground),
        Row::Task {
            id,
            title,
            completed,
            important: is_important,
            overdue,
            metadata,
            selected,
            selection_mode,
            multi_selected,
            dark,
            accent,
        } => {
            let p = todo_controls::Palette::for_dark(dark);
            let complete = vm.clone();
            let open = vm.clone();
            let important = vm.clone();
            let context = vm.clone();
            let select = vm.clone();
            let modifiers = vm.clone();
            let title_color = if completed { p.secondary } else { p.text };
            let accessible_title = title.clone();
            let selection_label = format!("Select {title}");
            let mut title_block = text_block()
                .text(title)
                .font_size(14.0)
                .foreground(title_color)
                .text_wrapping(TextWrapping::Wrap)
                .max_lines(2);
            if completed {
                title_block = title_block
                    .text_decorations(ferroui_base::media::TextDecorations::strikethrough());
            }
            todo_controls::task_card(
                grid()
                    .tag(super::interactions::task_tag(id))
                    .column_definitions(if selection_mode {
                        "34,42,*,40"
                    } else {
                        "0,42,*,40"
                    })
                    .min_height(54.0)
                    .on_context_requested(move |event| {
                        event.set_handled(true);
                        context.show_panel(Panel::TaskMenu(id));
                    })
                    .child(if selection_mode {
                        check_box()
                            .is_checked(Some(multi_selected))
                            .min_width(0.0)
                            .padding(0.0)
                            .horizontal_alignment(H::Center)
                            .vertical_alignment(V::Center)
                            .automation_properties_name(selection_label)
                            .on_click(move |_| select.toggle_task_selection(id))
                            .into()
                    } else {
                        Element::Empty
                    })
                    .child(
                        border()
                            .grid_column(1)
                            .vertical_alignment(V::Center)
                            .horizontal_alignment(H::Center)
                            .child(task_check(completed, &accent, move || {
                                complete.dispatch(Action::ToggleCompleted(id))
                            })),
                    )
                    .child(
                        button()
                            .grid_column(2)
                            .tag(super::interactions::task_tag(id))
                            .class("todo-flat")
                            .automation_properties_name(accessible_title)
                            .padding((4.0, 10.0))
                            .horizontal_content_alignment(H::Stretch)
                            .vertical_content_alignment(V::Center)
                            .background("Transparent")
                            .with(move |button| {
                                super::interactions::attach(button, modifiers.clone())
                            })
                            .on_click(move |_| {
                                if open.selection_mode() {
                                    open.toggle_task_selection(id);
                                } else {
                                    open.select(Some(id));
                                }
                            })
                            .child(stack_panel().spacing(4.0).child(title_block).child(
                                if metadata.is_empty() {
                                    Element::Empty
                                } else {
                                    text_block()
                                        .text(metadata)
                                        .font_size(11.0)
                                        .foreground(if overdue { "#A84646" } else { p.secondary })
                                        .into()
                                },
                            )),
                    )
                    .child(border().grid_column(3).vertical_alignment(V::Center).child(
                        star_button(is_important, &accent, move || {
                            important.dispatch(Action::ToggleImportant(id))
                        }),
                    )),
                selected,
                dark,
            )
        }
    }
}
