use crate::palette;
use ferroui_base::input::Key;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_base::media::TextWrapping;
use ferroui_controls::TextBox;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{action_button, primary_button, text_button, Icon};
use todo_domain::Action;
use todo_state::{AppViewModel, Panel};

pub fn view(vm: Rc<AppViewModel>) -> Element {
    rerender(move || content(vm.clone()))
}

fn content(vm: Rc<AppViewModel>) -> Element {
    let panel = vm.panel();
    let p = palette(&vm);
    let value = match panel {
        Panel::RenameList(id) => vm
            .state()
            .list(id)
            .map(|l| l.name.clone())
            .unwrap_or_default(),
        Panel::RenameGroup(id) => vm
            .state()
            .group(id)
            .map(|g| g.name.clone())
            .unwrap_or_default(),
        _ => String::new(),
    };
    let field = use_hook(NodeRef::<TextBox>::new);
    let cancel = vm.clone();
    let save_vm = vm.clone();
    let save: Rc<dyn Fn()> = Rc::new(move || {
        let value = field
            .get_untracked()
            .and_then(|f| f.text())
            .unwrap_or_default();
        if value.trim().is_empty() {
            save_vm.report_error("Enter a name to continue".into());
            return;
        }
        let action = match panel {
            Panel::NewList => Action::AddList { name: value },
            Panel::NewGroup => Action::AddGroup { name: value },
            Panel::RenameList(id) => Action::RenameList { id, name: value },
            Panel::RenameGroup(id) => Action::RenameGroup { id, name: value },
            _ => return,
        };
        save_vm.dispatch(action);
        save_vm.show_panel(Panel::None);
    });
    let enter = save.clone();
    let mut content = stack_panel()
        .spacing(18.0)
        .margin(8.0)
        .child(
            text_box()
                .name("ListName")
                .automation_properties_name("List or group name")
                .with(super::focus::name_field)
                .text(value)
                .placeholder_text("Enter a name")
                .node_ref(field)
                .font_size(15.0)
                .on_key_down(move |event| {
                    if event.key == Key::Return && !crate::editing::is_composing(event) {
                        enter();
                        event.set_handled(true);
                    }
                }),
        )
        .child(
            stack_panel()
                .orientation(Orientation::Horizontal)
                .horizontal_alignment(H::Right)
                .spacing(8.0)
                .child(text_button("Cancel", move || {
                    cancel.show_panel(Panel::None)
                }))
                .child(primary_button("Save", move || save())),
        );
    if let Panel::RenameGroup(id) = vm.panel() {
        let remove = vm.clone();
        content = content
            .child(
                text_block()
                    .text("Ungrouping keeps all of the lists and their tasks.")
                    .font_size(12.0)
                    .foreground(p.secondary)
                    .text_wrapping(TextWrapping::Wrap),
            )
            .child(action_button(Icon::Folder, "Ungroup lists", move || {
                remove.dispatch(Action::DeleteGroup(id));
                remove.show_panel(Panel::None);
            }));
    }
    content.into()
}
