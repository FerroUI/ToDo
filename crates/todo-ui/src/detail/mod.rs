mod action_row;
mod attachments;
mod fields;
mod new_step;
mod schedule_menu;
mod scheduling;
mod step_menu;
mod steps;

use self::attachments::attachments;
use self::fields::section;
use self::steps::steps;
use crate::palette;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::TextWrapping;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
pub(crate) use fields::edit_field;
use std::rc::Rc;
use todo_controls::{icon_button, star_button, task_check, Icon};
use todo_domain::{Action, Id};
use todo_state::{AppViewModel, Panel};

pub fn view(vm: Rc<AppViewModel>, id: Id) -> Element {
    let Some(task) = vm.state().task(id).cloned() else {
        return Element::Empty;
    };
    let p = palette(&vm);
    let close = vm.clone();
    let menu = vm.clone();
    let delete = vm.clone();
    let completed = vm.clone();
    let starred = vm.clone();
    let rename = vm.clone();
    let mut content = stack_panel()
        .spacing(10.0)
        .margin((12.0, 0.0, 12.0, 12.0))
        .child(
            border()
                .background(p.surface)
                .corner_radius(4.0)
                .padding((4.0, 10.0))
                .child(
                    stack_panel()
                        .spacing(4.0)
                        .child(
                            grid()
                                .column_definitions("40,*,36")
                                .child(border().vertical_alignment(V::Top).child(task_check(
                                    task.completed,
                                    p.accent,
                                    move || completed.dispatch(Action::ToggleCompleted(id)),
                                )))
                                .child(
                                    border().grid_column(1).child(
                                        edit_field(
                                            "TaskTitle",
                                            task.title.clone(),
                                            "Task name",
                                            true,
                                            move |title| {
                                                if !title.trim().is_empty() {
                                                    rename
                                                        .dispatch(Action::RenameTask { id, title });
                                                }
                                            },
                                        )
                                        .foreground(
                                            if task.completed { p.secondary } else { p.text },
                                        ),
                                    ),
                                )
                                .child(border().grid_column(2).vertical_alignment(V::Top).child(
                                    star_button(task.important, p.accent, move || {
                                        starred.dispatch(Action::ToggleImportant(id))
                                    }),
                                )),
                        )
                        .child(steps(vm.clone(), &task)),
                ),
        );
    let day = vm.clone();
    let remove_day = vm.clone();
    let included = task.is_my_day(vm.today());
    content = content.child(section(
        vm.clone(),
        action_row::row(
            action_row::action(
                Icon::Sun,
                if included {
                    "Added to My Day"
                } else {
                    "Add to My Day"
                },
                if included { p.accent } else { p.secondary },
            )
            .name("TaskMyDay")
            .on_click(move |event| {
                day.dispatch(Action::SetMyDay {
                    id,
                    day: if included { None } else { Some(day.today()) },
                });
                event.set_handled(true);
            }),
            included.then(|| {
                action_row::clear_button(
                    "Remove from My Day",
                    p.secondary,
                    "TaskMyDay",
                    move || remove_day.dispatch(Action::SetMyDay { id, day: None }),
                )
            }),
        ),
    ));
    content = content.child(scheduling::view(vm.clone(), &task));
    if task.assigned_to.is_some() {
        let assignee = task.assigned_to.clone().unwrap_or_default();
        content = content.child(section(
            vm.clone(),
            text_block()
                .text(format!("Assigned to {assignee}"))
                .font_size(14.0)
                .text_wrapping(TextWrapping::Wrap)
                .margin(16.0)
                .foreground(p.text),
        ));
    }
    content = content.child(attachments(vm.clone(), &task));
    let note = vm.clone();
    content = content.child(section(
        vm.clone(),
        border()
            .min_height(110.0)
            .padding((6.0, 8.0))
            .child(edit_field(
                "TaskNotes",
                task.notes.clone(),
                "Add a note",
                true,
                move |notes| note.dispatch(Action::SetNotes { id, notes }),
            )),
    ));
    let move_task = vm.clone();
    let list_name = vm
        .state()
        .list(task.list_id)
        .map(|list| list.name.clone())
        .unwrap_or_default();
    content = content.child(
        action_row::action(Icon::List, &format!("Move from {list_name}"), p.secondary).on_click(
            move |event| {
                move_task.show_panel(Panel::MoveTask(id));
                event.set_handled(true);
            },
        ),
    );
    border()
        .key(format!("task-detail-{id}"))
        .background(p.background)
        .border_brush(p.border)
        .border_thickness((1.0, 0.0, 0.0, 0.0))
        .child(
            grid()
                .row_definitions("48,*,48")
                .child(
                    grid()
                        .column_definitions("Auto,*,Auto")
                        .margin((8.0, 6.0))
                        .child(icon_button(
                            if vm.layout().compact {
                                Icon::ArrowLeft
                            } else {
                                Icon::ArrowRight
                            },
                            "Close details",
                            move || close.select(None),
                        ))
                        .child(
                            border().grid_column(2).child(
                                todo_controls::toolbar_button(
                                    Icon::More,
                                    "Task options",
                                    p.secondary,
                                )
                                .with(move |button| {
                                    crate::menus::task_options(button, menu.clone())
                                }),
                            ),
                        ),
                )
                .child(
                    scroll_viewer()
                        .grid_row(1)
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                        .child(content),
                )
                .child(
                    grid()
                        .grid_row(2)
                        .column_definitions("*,40")
                        .margin((16.0, 4.0))
                        .child(
                            text_block()
                                .text(format!("Created {}", task.created_on.format("%-d %B")))
                                .font_size(12.0)
                                .foreground(p.secondary)
                                .vertical_alignment(V::Center)
                                .horizontal_alignment(H::Center),
                        )
                        .child(border().grid_column(1).child(icon_button(
                            Icon::Trash,
                            "Delete task",
                            move || delete.delete_task(id),
                        ))),
                ),
        )
        .into()
}
