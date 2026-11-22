use super::item::{command, submenu};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::View;
use todo_state::{AppViewModel, Panel};

pub(super) fn items(vm: Rc<AppViewModel>) -> Vec<Element> {
    let theme = vm.clone();
    let completed = vm.clone();
    let print = vm.clone();
    let select = vm.clone();
    let mut items = vec![
        submenu("Sort by", Icon::Sort, super::sort::items(vm.clone())),
        command("Change theme", Icon::Sun, move || {
            theme.show_panel(Panel::Themes)
        }),
        command(
            if vm.completed_expanded() {
                "Hide completed tasks"
            } else {
                "Show completed tasks"
            },
            Icon::Check,
            move || completed.toggle_completed(),
        ),
        command("Select tasks", Icon::CheckCircle, move || {
            select.begin_selection()
        }),
        separator().into(),
        command("Print list", Icon::Print, move || print.print_list()),
    ];
    if let View::List(id) = vm.view() {
        let share = vm.clone();
        items.push(command("Share list", Icon::Share, move || {
            share.show_panel(Panel::Share)
        }));
        items.push(separator().into());
        items.extend(super::list::items(vm, id));
    }
    items
}
