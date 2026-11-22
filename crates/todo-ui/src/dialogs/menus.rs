use crate::palette;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{ALL_LIST_THEMES, Icon, action_button};
use todo_domain::{Action, Id, SortOrder, TASKS_LIST, View};
use todo_state::{AppViewModel, Panel};

pub fn list(vm: Rc<AppViewModel>) -> Element {
    let sort_vm = vm.clone();
    let theme_vm = vm.clone();
    let print_vm = vm.clone();
    let completed_vm = vm.clone();
    let mut options = stack_panel()
        .spacing(2.0)
        .child(action_button(Icon::Sort, "Sort by", move || {
            sort_vm.show_panel(Panel::Sort)
        }))
        .child(action_button(Icon::Sun, "Change theme", move || {
            theme_vm.show_panel(Panel::Themes)
        }))
        .child(action_button(
            Icon::Check,
            if vm.completed_expanded() {
                "Hide completed tasks"
            } else {
                "Show completed tasks"
            },
            move || {
                completed_vm.toggle_completed();
                completed_vm.show_panel(Panel::None);
            },
        ))
        .child(action_button(Icon::Print, "Print list", move || {
            print_vm.print_list();
            print_vm.show_panel(Panel::None);
        }));
    if let View::List(id) = vm.view() {
        let duplicate = vm.clone();
        let share = vm.clone();
        options = options
            .child(action_button(Icon::Copy, "Duplicate list", move || {
                duplicate.dispatch(Action::DuplicateList(id));
                duplicate.show_panel(Panel::None);
            }))
            .child(action_button(Icon::Share, "Share list", move || {
                share.show_panel(Panel::Share)
            }));
        if id != TASKS_LIST {
            let rename = vm.clone();
            let remove = vm.clone();
            let group = vm.clone();
            options = options
                .child(action_button(Icon::Note, "Rename list", move || {
                    rename.show_panel(Panel::RenameList(id))
                }))
                .child(action_button(Icon::Folder, "Move to group", move || {
                    group.show_panel(Panel::MoveList(id))
                }))
                .child(action_button(Icon::Trash, "Delete list", move || {
                    remove.show_panel(Panel::DeleteList(id))
                }));
        }
    }
    options.into()
}

pub fn sort(vm: Rc<AppViewModel>) -> Element {
    let mut content = stack_panel().spacing(2.0);
    for order in [
        SortOrder::Manual,
        SortOrder::Importance,
        SortOrder::DueDate,
        SortOrder::Alphabetical,
        SortOrder::Created,
        SortOrder::MyDay,
    ] {
        let choose = vm.clone();
        content = content.child(action_button(
            if vm.sort() == order {
                Icon::Check
            } else {
                Icon::Sort
            },
            order.label(),
            move || choose.set_sort(order),
        ));
    }
    content.into()
}

pub fn themes(vm: Rc<AppViewModel>) -> Element {
    let p = palette(&vm);
    let selected_accent = vm.accent();
    let mut swatches = wrap_panel()
        .orientation(Orientation::Horizontal)
        .item_width(104.0)
        .item_height(76.0);
    for theme in ALL_LIST_THEMES {
        let select = vm.clone();
        let color = theme.accent.to_string();
        swatches = swatches.child(
            button()
                .class("todo-flat")
                .width(94.0)
                .height(68.0)
                .margin(4.0)
                .padding(0.0)
                .border_thickness(2.0)
                .border_brush(if selected_accent.eq_ignore_ascii_case(theme.accent) {
                    p.accent
                } else {
                    "Transparent"
                })
                .automation_properties_name(format!("{} list theme", theme.name))
                .tool_tip_tip(theme.name)
                .on_click(move |_| {
                    if let View::List(id) = select.view() {
                        select.dispatch(Action::SetListTheme {
                            id,
                            color: color.clone(),
                            background: None,
                        });
                        select.show_panel(Panel::None);
                    } else {
                        select.set_smart_color(color.clone());
                        select.show_panel(Panel::None);
                    }
                })
                .child(
                    border()
                        .background(if vm.is_dark() {
                            theme.dark_background
                        } else {
                            theme.background
                        })
                        .corner_radius(4.0)
                        .padding(10.0)
                        .child(
                            stack_panel()
                                .spacing(7.0)
                                .child(
                                    border()
                                        .height(5.0)
                                        .width(54.0)
                                        .horizontal_alignment(H::Left)
                                        .background(theme.accent)
                                        .corner_radius(2.0),
                                )
                                .child(
                                    text_block()
                                        .text(theme.name)
                                        .font_size(12.0)
                                        .foreground(p.text),
                                ),
                        ),
                ),
        );
    }
    stack_panel()
        .spacing(12.0)
        .child(
            text_block()
                .text("Choose a color for this list")
                .font_size(13.0)
                .foreground(p.secondary)
                .margin(8.0),
        )
        .child(swatches)
        .into()
}

pub fn task(vm: Rc<AppViewModel>, id: Id) -> Element {
    let Some(task) = vm.state().task(id).cloned() else {
        return Element::Empty;
    };
    let day = vm.clone();
    let important = vm.clone();
    let complete = vm.clone();
    let due = vm.clone();
    let move_task = vm.clone();
    let duplicate = vm.clone();
    let remove = vm.clone();
    let up = vm.clone();
    let down = vm.clone();
    let included = task.is_my_day(vm.today());
    let mut menu = stack_panel()
        .spacing(2.0)
        .child(action_button(
            Icon::Sun,
            if included {
                "Remove from My Day"
            } else {
                "Add to My Day"
            },
            move || {
                day.dispatch(Action::SetMyDay {
                    id,
                    day: if included { None } else { Some(day.today()) },
                });
                day.show_panel(Panel::None);
            },
        ))
        .child(action_button(
            Icon::Star,
            if task.important {
                "Remove importance"
            } else {
                "Mark as important"
            },
            move || {
                important.dispatch(Action::ToggleImportant(id));
                important.show_panel(Panel::None);
            },
        ))
        .child(action_button(
            Icon::Check,
            if task.completed {
                "Mark as not completed"
            } else {
                "Mark as completed"
            },
            move || {
                complete.dispatch(Action::ToggleCompleted(id));
                complete.show_panel(Panel::None);
            },
        ))
        .child(action_button(Icon::Calendar, "Set due date", move || {
            due.show_panel(Panel::DueDate(id))
        }))
        .child(action_button(Icon::List, "Move task to…", move || {
            move_task.show_panel(Panel::MoveTask(id))
        }))
        .child(action_button(Icon::Copy, "Duplicate task", move || {
            duplicate.dispatch(Action::DuplicateTask(id));
            duplicate.show_panel(Panel::None);
        }));
    if vm.sort() == SortOrder::Manual {
        menu = menu
            .child(action_button(Icon::ArrowUp, "Move up", move || {
                up.move_task_by(id, -1);
                up.show_panel(Panel::None);
            }))
            .child(action_button(Icon::ArrowDown, "Move down", move || {
                down.move_task_by(id, 1);
                down.show_panel(Panel::None);
            }));
    }
    menu.child(action_button(Icon::Trash, "Delete task", move || {
        remove.delete_task(id)
    }))
    .into()
}
