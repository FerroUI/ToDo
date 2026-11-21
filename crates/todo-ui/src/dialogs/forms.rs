pub use super::{help::view as help, name::view as name};

use crate::palette;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{action_button, primary_button, text_button, Icon};
use todo_domain::Action;
use todo_state::{AppViewModel, HostRequest, Panel};

pub fn confirm(vm: Rc<AppViewModel>) -> Element {
    let panel = vm.panel();
    let p = palette(&vm);
    let cancel = vm.clone();
    let remove = vm.clone();
    let description = match panel {
        Panel::DeleteTask(id) => format!(
            "\"{}\" will be deleted. You can undo this change.",
            vm.state()
                .task(id)
                .map(|t| t.title.clone())
                .unwrap_or_default()
        ),
        Panel::DeleteList(id) => format!(
            "\"{}\" and its tasks will be deleted. You can undo this change.",
            vm.state()
                .list(id)
                .map(|l| l.name.clone())
                .unwrap_or_default()
        ),
        Panel::DeleteSelection => format!(
            "{} selected tasks will be deleted. You can undo this change.",
            vm.selection_count()
        ),
        _ => String::new(),
    };
    stack_panel()
        .spacing(20.0)
        .margin(8.0)
        .child(
            text_block()
                .text(description)
                .font_size(14.0)
                .foreground(p.text)
                .text_wrapping(TextWrapping::Wrap),
        )
        .child(
            stack_panel()
                .orientation(Orientation::Horizontal)
                .horizontal_alignment(H::Right)
                .spacing(8.0)
                .child(text_button("Cancel", move || {
                    cancel.show_panel(Panel::None)
                }))
                .child(primary_button("Delete", move || match panel {
                    Panel::DeleteTask(id) => remove.confirm_delete_task(id),
                    Panel::DeleteSelection => remove.confirm_delete_selection(),
                    Panel::DeleteList(id) => {
                        remove.dispatch(Action::DeleteList(id));
                        remove.show_panel(Panel::None);
                    }
                    _ => {}
                })),
        )
        .into()
}

pub fn move_to(vm: Rc<AppViewModel>) -> Element {
    let mut content = stack_panel().spacing(2.0);
    match vm.panel() {
        Panel::MoveSelection => {
            for list in vm.state().lists.clone() {
                let move_vm = vm.clone();
                let list_id = list.id;
                content = content.child(action_button(Icon::List, &list.name, move || {
                    move_vm.move_selection(list_id)
                }));
            }
        }
        Panel::MoveTask(id) => {
            let lists = vm.state().lists.clone();
            for list in lists {
                let move_vm = vm.clone();
                let list_id = list.id;
                content = content.child(action_button(Icon::List, &list.name, move || {
                    move_vm.dispatch(Action::MoveTask { id, list_id });
                    move_vm.show_panel(Panel::None);
                }));
            }
        }
        Panel::MoveList(id) => {
            let none = vm.clone();
            content = content.child(action_button(Icon::List, "Outside a group", move || {
                none.dispatch(Action::MoveList { id, group_id: None });
                none.show_panel(Panel::None);
            }));
            for group in vm.state().groups.clone() {
                let move_vm = vm.clone();
                let group_id = Some(group.id);
                content = content.child(action_button(Icon::Folder, &group.name, move || {
                    move_vm.dispatch(Action::MoveList { id, group_id });
                    move_vm.show_panel(Panel::None);
                }));
            }
        }
        _ => {}
    }
    content.into()
}

pub fn share(vm: Rc<AppViewModel>) -> Element {
    let p = palette(&vm);
    let export = vm.clone();
    let open = vm.clone();
    stack_panel().spacing(14.0).margin(8.0)
        .child(text_block().text("Share a copy of your tasks").font_size(16.0).foreground(p.text))
        .child(text_block().text("Export a backup to transfer your lists. Live Microsoft list invitations and access permissions are managed in Microsoft To Do.")
            .font_size(13.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
        .child(action_button(Icon::Download, "Export backup", move || export.export_backup()))
        .child(action_button(Icon::Share, "Open Microsoft To Do", move || open.request(HostRequest::OpenUrl { url: "https://to-do.office.com/tasks/".into() })))
        .into()
}
