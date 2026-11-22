use super::controls::{section_label, toggle};
use crate::palette;
use ferroui_base::media::TextWrapping;
use ferroui_controls::TextBox;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{action_button, primary_button, Icon};
use todo_domain::Action;
use todo_state::{AppViewModel, HostRequest};

pub(super) fn view(vm: Rc<AppViewModel>) -> Element {
    let field = use_hook(NodeRef::<TextBox>::new);
    let p = palette(&vm);
    let settings = vm.state().settings.clone();
    let mut content = stack_panel()
        .spacing(8.0)
        .child(section_label("Microsoft account", p.text));
    if !vm.cloud_supported() {
        let open = vm.clone();
        return content
            .child(
                text_block()
                    .text("Connect Microsoft To Do and Planner in the browser app.")
                    .font_size(12.0)
                    .foreground(p.secondary)
                    .text_wrapping(TextWrapping::Wrap),
            )
            .child(action_button(Icon::Person, "Open browser app", move || {
                open.request(HostRequest::OpenUrl {
                    url: "https://ferroui.com/ToDo/".into(),
                })
            }))
            .into();
    }
    if vm.cloud_connected() {
        let sync = vm.clone();
        let reconnect = vm.clone();
        let disconnect = vm.clone();
        let sign_out = vm.clone();
        content = content
            .child(
                text_block()
                    .text(vm.cloud_label())
                    .font_size(14.0)
                    .foreground(p.text)
                    .text_wrapping(TextWrapping::Wrap),
            )
            .child(action_button(Icon::Repeat, "Sync now", move || {
                sync.request(HostRequest::SyncMicrosoft)
            }))
            .child(toggle(
                vm.clone(),
                "Sync automatically",
                settings.auto_sync,
                |s, value| s.auto_sync = value,
            ))
            .child(action_button(
                Icon::Person,
                "Reconnect Microsoft account",
                move || reconnect.request(HostRequest::ReconnectMicrosoft),
            ))
            .child(action_button(
                Icon::Close,
                "Disconnect this app",
                move || disconnect.request(HostRequest::DisconnectMicrosoft),
            ))
            .child(action_button(
                Icon::Person,
                "Sign out of Microsoft",
                move || sign_out.request(HostRequest::SignOutMicrosoft),
            ));
        if vm.cloud_work_account() {
            content = content.child(section_label("Connected apps", p.text))
                .child(toggle(vm.clone(), "Planner", settings.planner_enabled, |s, value| {
                    s.planner_enabled = value;
                    if value { s.show_assigned = true; }
                }))
                .child(text_block().text("Show assigned Planner tasks in Assigned to me. Changes to their title, due date, notes, steps and completion sync with Planner.")
                    .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap));
        }
    } else {
        let connect = vm.clone();
        let configured = vm.cloud_configured();
        if !configured {
            content = content.child(text_block().text("This deployment needs a Microsoft application registration. Enter its public client ID; the sign-in address must be registered for this site.")
                .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
                .child(text_box().name("MicrosoftClientId").automation_properties_name("Application client ID")
                    .text(settings.microsoft_client_id.clone()).placeholder_text("Application (client) ID")
                    .node_ref(field).font_size(14.0));
        }
        content = content.child(text_block().text("Connect your personal or work account to sync your tasks and files with Microsoft To Do and Outlook.")
                .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
            .child(primary_button("Connect Microsoft account", move || {
                let client_id = if configured { settings.microsoft_client_id.clone() }
                    else { field.get_untracked().and_then(|input| input.text()).unwrap_or_default().trim().to_owned() };
                if !configured && client_id.is_empty() {
                    connect.report_error("Enter your registered application's client ID".into()); return;
                }
                let mut settings = connect.state().settings.clone();
                settings.microsoft_client_id = client_id.clone();
                connect.dispatch(Action::SetSettings(settings));
                connect.request(HostRequest::ConnectMicrosoft { client_id });
            }));
        if !vm.cloud_label().is_empty() && vm.cloud_label() != "Local account" {
            content = content.child(
                text_block()
                    .text(vm.cloud_label())
                    .font_size(12.0)
                    .foreground(p.secondary)
                    .text_wrapping(TextWrapping::Wrap),
            );
        }
    }
    content.into()
}
