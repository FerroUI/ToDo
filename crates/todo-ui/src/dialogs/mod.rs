mod dates;
mod focus;
mod forms;
mod help;
mod menus;
mod name;
mod step_menu;

use crate::palette;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{icon_button, Icon};
use todo_state::{AppViewModel, Panel};

pub fn view(vm: Rc<AppViewModel>) -> Element {
    let p = palette(&vm);
    let panel = vm.panel();
    let panel_key = format!("{panel:?}");
    let dismiss = vm.clone();
    let close = vm.clone();
    let title = match &panel {
        Panel::NewList => "New list",
        Panel::NewGroup => "New group",
        Panel::RenameList(_) => "Rename list",
        Panel::RenameGroup(_) => "Edit group",
        Panel::MoveTask(_) | Panel::MoveSelection => "Move tasks to",
        Panel::MoveList(_) => "Move list to group",
        Panel::DueDate(_) => "Due date",
        Panel::Reminder(_) => "Remind me",
        Panel::Repeat(_) => "Repeat",
        Panel::DeleteTask(_) => "Delete task?",
        Panel::DeleteSelection => "Delete selected tasks?",
        Panel::DeleteList(_) => "Delete list?",
        Panel::Sort => "Sort by",
        Panel::Themes => "Change theme",
        Panel::TaskMenu(_) => "Task options",
        Panel::StepMenu { .. } => "Step options",
        Panel::Help => "Keyboard shortcuts",
        Panel::Share => "Share list",
        _ => "List options",
    };
    let content = match panel {
        Panel::DueDate(id) => dates::date_picker(vm.clone(), id, false),
        Panel::Reminder(id) => dates::date_picker(vm.clone(), id, true),
        Panel::Repeat(id) => dates::repeat(vm.clone(), id),
        Panel::NewList | Panel::NewGroup | Panel::RenameList(_) | Panel::RenameGroup(_) => {
            forms::name(vm.clone())
        }
        Panel::DeleteTask(_) | Panel::DeleteList(_) | Panel::DeleteSelection => {
            forms::confirm(vm.clone())
        }
        Panel::MoveTask(_) | Panel::MoveList(_) | Panel::MoveSelection => {
            forms::move_to(vm.clone())
        }
        Panel::Sort => menus::sort(vm.clone()),
        Panel::Themes => menus::themes(vm.clone()),
        Panel::TaskMenu(id) => menus::task(vm.clone(), id),
        Panel::StepMenu { task_id, step_id } => step_menu::view(vm.clone(), task_id, step_id),
        Panel::Help => forms::help(),
        Panel::Share => forms::share(vm.clone()),
        _ => menus::list(vm.clone()),
    };
    grid()
        .child(
            button()
                .focusable(false)
                .is_tab_stop(false)
                .class("todo-flat")
                .background("#55000000")
                .border_thickness(0.0)
                .horizontal_alignment(H::Stretch)
                .vertical_alignment(V::Stretch)
                .on_click(move |_| dismiss.show_panel(Panel::None)),
        )
        .child(
            border()
                .key(panel_key)
                .keyboard_navigation_tab_navigation(
                    ferroui_base::input::KeyboardNavigationMode::Cycle,
                )
                .automation_properties_name(title)
                .with(focus::activate)
                .background(p.surface)
                .border_brush(p.border)
                .border_thickness(1.0)
                .corner_radius(8.0)
                .max_width(if vm.is_narrow() { 320.0 } else { 360.0 })
                .max_height(680.0)
                .horizontal_alignment(H::Stretch)
                .vertical_alignment(V::Center)
                .margin(16.0)
                .child(
                    grid()
                        .row_definitions("Auto,*")
                        .child(
                            grid()
                                .column_definitions("*,Auto")
                                .margin((20.0, 12.0, 12.0, 6.0))
                                .child(
                                    text_block()
                                        .text(title)
                                        .font_size(18.0)
                                        .font_weight(FontWeight::SemiBold)
                                        .foreground(p.text)
                                        .vertical_alignment(V::Center),
                                )
                                .child(border().grid_column(1).child(icon_button(
                                    Icon::Close,
                                    "Close",
                                    move || close.show_panel(Panel::None),
                                ))),
                        )
                        .child(
                            scroll_viewer()
                                .grid_row(1)
                                .max_height(600.0)
                                .child(border().padding((12.0, 4.0, 12.0, 16.0)).child(content)),
                        ),
                ),
        )
        .into()
}
