use crate::{palette, shell::Drafts};
use ferroui_base::input::Key;
use ferroui_base::layout::VerticalAlignment as V;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{Icon, icon, icon_button};
use todo_state::AppViewModel;
pub(super) fn view(vm: Rc<AppViewModel>, my_day: bool, drafts: Rc<Drafts>) -> Element {
    rerender(move || content(vm.clone(), my_day, drafts.clone()))
}

fn content(vm: Rc<AppViewModel>, my_day: bool, drafts: Rc<Drafts>) -> Element {
    let p = palette(&vm);
    let draft = drafts.task;
    let add = vm.clone();
    let enter = vm.clone();
    let keyboard = drafts.clone();
    let submit = drafts.clone();
    border()
        .corner_radius(5.0)
        .background(if my_day {
            if vm.is_dark() {
                "#E52B3241"
            } else {
                "#F2FFFFFF"
            }
        } else {
            p.surface
        })
        .min_height(54.0)
        .padding((10.0, 4.0))
        .child(
            grid()
                .column_definitions("36,*,36")
                .row_definitions("Auto,Auto")
                .child(border().vertical_alignment(V::Center).child(icon(
                    Icon::Add,
                    22.0,
                    p.accent,
                )))
                .child(
                    text_box()
                        .name("NewTask")
                        .automation_properties_name("Add a task")
                        .automation_properties_help_text("Type a task and press Enter to add it.")
                        .node_ref(drafts.task_input)
                        .grid_column(1)
                        .class("todo-input")
                        .text(draft.two_way())
                        .placeholder_text("Add a task")
                        .font_size(14.0)
                        .background("Transparent")
                        .border_thickness(0.0)
                        .vertical_alignment(V::Center)
                        .padding((4.0, 9.0))
                        .foreground(p.text)
                        .on_key_down(move |event| {
                            if event.key == Key::Return {
                                keyboard.submit_task(&enter);
                                event.set_handled(true);
                            }
                        }),
                )
                .child(
                    border()
                        // Hints must never take width from the editor in a narrow list pane.
                        .grid_column(1)
                        .grid_row(1)
                        .vertical_alignment(V::Center)
                        .child(super::entry_hint::view(&vm, &draft.get())),
                )
                .child(
                    border()
                        .grid_column(2)
                        .vertical_alignment(V::Center)
                        .child(icon_button(Icon::ArrowUp, "Add task", move || {
                            submit.submit_task(&add);
                        })),
                ),
        )
        .into()
}
