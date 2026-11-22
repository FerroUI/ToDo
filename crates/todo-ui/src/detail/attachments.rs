use super::{
    action_row::{action, clear_button, row},
    fields::section,
};
use crate::palette;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::{Action, Task};
use todo_state::{AppViewModel, HostRequest};

pub(super) fn attachments(vm: Rc<AppViewModel>, task: &Task) -> Element {
    let id = task.id;
    let p = palette(&vm);
    let attach = vm.clone();
    let mut files = stack_panel().child(
        action(Icon::Attachment, "Add file", p.secondary)
            .name("AddAttachment")
            .on_click(move |event| {
                attach.request(HostRequest::Attach { task_id: id });
                event.set_handled(true);
            }),
    );
    for file in &task.attachments {
        let open = vm.clone();
        let remove = vm.clone();
        let key = file.storage_key.clone();
        let attachment_id = file.id;
        files = files.child(row(
            action(Icon::Attachment, &file.name, p.accent)
                .tool_tip_tip(file.name.clone())
                .on_click(move |event| {
                    open.request(HostRequest::OpenAttachment { key: key.clone() });
                    event.set_handled(true);
                }),
            Some(clear_button(
                &format!("Remove {}", file.name),
                p.secondary,
                "AddAttachment",
                move || {
                    // Blob retention allows undo to restore a removed attachment. Storage cleanup is explicit.
                    remove.dispatch(Action::DeleteAttachment {
                        task_id: id,
                        attachment_id,
                    });
                },
            )),
        ));
    }
    section(vm, files)
}
