use super::{counts::Counts, group, item::view as nav_item};
use crate::palette;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::{SidebarItem, View, TASKS_LIST};
use todo_state::AppViewModel;

pub(super) fn view(vm: Rc<AppViewModel>) -> Element {
    let p = palette(&vm);
    let state = vm.state();
    let counts = Counts::new(&state, vm.today());
    let show_all = state.settings.show_all;
    let show_completed = state.settings.show_completed_list;
    let show_assigned = state.settings.show_assigned;
    let show_flagged = state.settings.show_flagged_email;
    let mut lists = state.lists.clone();
    lists.sort_by_key(|l| (l.position, l.id));
    let groups = state.groups.clone();
    let roots = state.sidebar_items();
    drop(state);
    let keys = vm.clone();
    let mut nav = stack_panel()
        .name("ListNavigation")
        .spacing(1.0)
        .margin((8.0, 4.0))
        .with(move |nav| super::keyboard::attach(nav, keys.clone()));
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
    for entry in roots {
        match entry {
            SidebarItem::List(id) => {
                if let Some(list) = lists.iter().find(|list| list.id == id) {
                    nav = nav.child(nav_item(
                        vm.clone(),
                        View::List(id),
                        Icon::List,
                        &list.color,
                        &counts,
                    ));
                }
            }
            SidebarItem::Group(id) => {
                if let Some(group) = groups.iter().find(|group| group.id == id) {
                    nav = nav.child(group::header(vm.clone(), group));
                    if !group.collapsed {
                        let members: Vec<_> = lists
                            .iter()
                            .filter(|list| list.group_id == Some(id))
                            .collect();
                        if members.is_empty() {
                            nav = nav.child(group::empty(vm.clone(), id));
                        }
                        for list in members {
                            nav =
                                nav.child(border().margin((16.0, 0.0, 0.0, 0.0)).child(nav_item(
                                    vm.clone(),
                                    View::List(list.id),
                                    Icon::List,
                                    &list.color,
                                    &counts,
                                )));
                        }
                    }
                }
            }
        }
    }
    nav.child(
        button()
            .name("SidebarEnd")
            .class("todo-flat")
            .height(16.0)
            .vertical_content_alignment(ferroui_base::layout::VerticalAlignment::Stretch)
            .focusable(false)
            .is_tab_stop(false)
            .automation_properties_name("End of lists")
            .with(move |button| {
                crate::drag::attach_sidebar_target(
                    button,
                    vm.clone(),
                    crate::drag::SidebarTarget::End,
                )
            })
            .child(grid().child(crate::drag::sidebar_marker(p.accent, true))),
    )
    .into()
}
