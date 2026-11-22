mod account;
mod counts;
mod group;
mod item;
mod keyboard;
mod list_icon;
mod navigation;
mod search;

use crate::palette;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{Icon, icon, icon_button};
use todo_state::AppViewModel;

pub fn view(vm: Rc<AppViewModel>, drafts: Rc<crate::shell::Drafts>) -> Element {
    let p = palette(&vm);
    let new_list = vm.clone();
    let new_group = vm.clone();
    border()
        .background(p.sidebar)
        .border_brush(p.border)
        .border_thickness((0.0, 0.0, 1.0, 0.0))
        .child(
            grid()
                .row_definitions("Auto,Auto,*,Auto")
                .child(account::view(vm.clone()))
                .child(search::view(vm.clone(), drafts).grid_row(1))
                .child(
                    scroll_viewer()
                        .grid_row(2)
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                        .child(navigation::view(vm.clone())),
                )
                .child(
                    grid()
                        .grid_row(3)
                        .column_definitions("*,44")
                        .height(52.0)
                        .margin((8.0, 0.0))
                        .child(
                            button()
                                .class("todo-flat")
                                .automation_properties_name("New list")
                                .foreground(p.accent)
                                .horizontal_content_alignment(H::Left)
                                .on_click(move |_| new_list.create_list_inline())
                                .child(
                                    stack_panel()
                                        .orientation(Orientation::Horizontal)
                                        .spacing(12.0)
                                        .child(icon(Icon::Add, 20.0, p.accent))
                                        .child(text_block().text("New list").font_size(14.0)),
                                ),
                        )
                        .child(border().grid_column(1).vertical_alignment(V::Center).child(
                            icon_button(Icon::Folder, "Create a new group", move || {
                                new_group.create_group_inline()
                            }),
                        )),
                ),
        )
        .into()
}
