use super::controls::section_label;
use crate::palette;
use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{action_button, Icon};
use todo_state::{AppViewModel, HostRequest, Panel};

pub(super) fn view(vm: Rc<AppViewModel>) -> Element {
    let p = palette(&vm);
    let export = vm.clone();
    let restore = vm.clone();
    let mut content = stack_panel()
        .spacing(8.0)
        .child(section_label("Your data", p.text));
    if vm.cloud_supported() {
        let portable = vm.clone();
        let import = vm.clone();
        let pst = vm.clone();
        let pst_export = vm.clone();
        content = content
            .child(action_button(
                Icon::Download,
                "Export backup with files",
                move || portable.request(HostRequest::ExportArchive),
            ))
            .child(action_button(
                Icon::Upload,
                "Restore Microsoft archive…",
                move || import.request(HostRequest::ImportMicrosoft),
            ))
            .child(action_button(
                Icon::Upload,
                "Import Outlook PST…",
                move || pst.request(HostRequest::ImportPst),
            ))
            .child(action_button(
                Icon::Download,
                "Export Outlook PST",
                move || pst_export.request(HostRequest::ExportPst),
            ));
        if vm.cloud_connected() {
            let microsoft = vm.clone();
            content = content.child(action_button(
                Icon::Download,
                "Export Microsoft To Do data",
                move || microsoft.request(HostRequest::ExportMicrosoft),
            ));
        }
    }
    let notify = vm.clone();
    let help = vm.clone();
    let background = vm.state().settings.background_reminders;
    let background_status = vm.background_reminders_status();
    if vm.background_reminders_supported() || background || !background_status.is_empty() {
        content=content.child(section_label("Background reminders",p.text))
            .child(border().is_enabled(vm.background_reminders_supported() || background).child(super::controls::toggle(
                vm.clone(),"Remind me after this app closes",background,|settings,value|settings.background_reminders=value)))
            .child(text_block().text(background_status).font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
            .child(text_block().text("Schedules reminders from the active account or local workspace. You must remain signed in to this computer. Sleeping computers catch up when available; a powered-off computer cannot show notifications.")
                .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap));
    }
    content.child(action_button(Icon::Download, "Export backup", move || export.export_backup()))
        .child(action_button(Icon::Upload, "Restore backup…", move || restore.request(HostRequest::Import)))
        .child(text_block().text("Backups with files are portable. Restoring makes a local copy and pauses Microsoft sync. Plain JSON backups contain file references only.")
            .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
        .child(action_button(Icon::Bell, "Enable notifications", move || notify.request(HostRequest::RequestNotifications)))
        .child(text_block().text(if background || vm.push_state().enabled {"Background reminder scheduling is enabled for this workspace."} else {"Reminders are delivered while this app is open."})
            .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
        .child(section_label("About", p.text))
        .child(action_button(Icon::Info, "Keyboard shortcuts", move || help.show_panel(Panel::Help)))
        .child(text_block().text("FerroUI To Do  ·  0.1.0\nBuilt with Rust and FerroUI")
            .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap).margin((0.0, 12.0))).into()
}
