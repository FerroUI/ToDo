use super::fields::section;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{action_button, icon_button, Icon};
use todo_domain::{Action, Task};
use todo_state::{AppViewModel, HostRequest};

pub(super) fn attachments(vm: Rc<AppViewModel>, task: &Task) -> Element {
    let id = task.id;
    let attach = vm.clone();
    let mut files = stack_panel().child(action_button(Icon::Attachment, "Add file", move || {
        attach.request(HostRequest::Attach { task_id: id })
    }));
    for file in &task.attachments {
        let open = vm.clone();
        let remove = vm.clone();
        let key = file.storage_key.clone();
        let attachment_id = file.id;
        files = files.child(
            grid()
                .column_definitions("*,32")
                .child(action_button(Icon::Attachment, &file.name, move || {
                    open.request(HostRequest::OpenAttachment { key: key.clone() })
                }))
                .child(border().grid_column(1).child(icon_button(
                    Icon::Close,
                    "Remove file",
                    move || {
                        // Blob retention allows undo to restore a removed attachment. Storage cleanup is explicit.
                        remove.dispatch(Action::DeleteAttachment {
                            task_id: id,
                            attachment_id,
                        });
                    },
                ))),
        );
    }
    section(vm, files)
}
