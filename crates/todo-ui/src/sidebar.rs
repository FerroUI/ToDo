mod account;
mod counts;
mod group;
mod item;
mod search;

use crate::palette;
use counts::Counts;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use item::view as nav_item;
use std::rc::Rc;
use todo_controls::{icon, icon_button, Icon};
use todo_domain::{View, TASKS_LIST};
use todo_state::AppViewModel;

pub fn view(vm: Rc<AppViewModel>, drafts: Rc<crate::shell::Drafts>) -> Element {
    let p = palette(&vm);
    let state = vm.state();
    let counts = Counts::new(&state, vm.today());
    let show_all = state.settings.show_all;
    let show_completed = state.settings.show_completed_list;
    let show_assigned = state.settings.show_assigned;
    let show_flagged = state.settings.show_flagged_email;
    let mut lists = state.lists.clone();
    lists.sort_by_key(|l| (l.position, l.id));
    let mut groups = state.groups.clone();
    groups.sort_by_key(|g| (g.position, g.id));
    drop(state);
    let new_list = vm.clone();
    let new_group = vm.clone();
    let mut nav = stack_panel().spacing(1.0).margin((8.0, 4.0));
    for (target, symbol, color) in [
        (View::MyDay, Icon::Sun, "#797775"),
        (View::Important, Icon::Star, "#B65B7E"),
        (View::Planned, Icon::Calendar, "#287F6F"),
    ] {
        nav = nav.child(nav_item(vm.clone(), target, symbol, color, &counts));
    }
    if show_assigned {
        nav = nav.child(nav_item(
            vm.clone(),
            View::AssignedToMe,
            Icon::Person,
            "#328591",
            &counts,
        ));
    }
    if show_flagged {
        nav = nav.child(nav_item(
            vm.clone(),
            View::FlaggedEmail,
            Icon::Flag,
            "#C96F38",
            &counts,
        ));
    }
    if show_all {
        nav = nav.child(nav_item(
            vm.clone(),
            View::All,
            Icon::Home,
            "#6B779F",
            &counts,
        ));
    }
    if show_completed {
        nav = nav.child(nav_item(
            vm.clone(),
            View::Completed,
            Icon::Check,
            "#398663",
            &counts,
        ));
    }
    nav = nav
        .child(nav_item(
            vm.clone(),
            View::List(TASKS_LIST),
            Icon::Home,
            "#2564CF",
            &counts,
        ))
        .child(
            border()
                .height(1.0)
                .background(p.border)
                .margin((8.0, 12.0)),
        );
    for list in lists
        .iter()
        .filter(|l| l.id != TASKS_LIST && l.group_id.is_none())
    {
        nav = nav.child(nav_item(
            vm.clone(),
            View::List(list.id),
            Icon::List,
            &list.color,
            &counts,
        ));
    }
    for group in groups {
        let id = group.id;
        nav = nav.child(group::header(vm.clone(), &group));
        if !group.collapsed {
            for list in lists.iter().filter(|l| l.group_id == Some(id)) {
                nav = nav.child(border().margin((16.0, 0.0, 0.0, 0.0)).child(nav_item(
                    vm.clone(),
                    View::List(list.id),
                    Icon::List,
                    &list.color,
                    &counts,
                )));
            }
        }
    }
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
                        .child(nav),
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
