mod controls;

use crate::{detail::edit_field, palette};
use controls::{section_label, toggle};
use ferroui_base::layout::VerticalAlignment as V;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::TextBox;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{action_button, icon_button, primary_button, Icon};
use todo_domain::{Action, Theme};
use todo_state::{AppViewModel, HostRequest, Panel};

pub fn view(vm: Rc<AppViewModel>) -> Element {
    rerender(move || content(vm.clone()))
}

fn content(vm: Rc<AppViewModel>) -> Element {
    let client_id_field = use_hook(NodeRef::<TextBox>::new);
    let p = palette(&vm);
    let settings = vm.state().settings.clone();
    let close = vm.clone();
    let name = vm.clone();
    let mut content = stack_panel().spacing(8.0).margin((20.0, 4.0, 20.0, 24.0))
        .child(section_label("Account", p.text))
        .child(edit_field("AccountName", settings.account_name.clone(), "Your name", false, move |account_name| {
            let mut settings = name.state().settings.clone(); settings.account_name = account_name; name.dispatch(Action::SetSettings(settings));
        }))
        .child(text_block().text("Your tasks are saved on this device. Export a backup to keep an independent copy.")
            .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap).margin((0.0, 0.0, 0.0, 8.0)))
        .child(section_label("Appearance", p.text));
    for (theme, label) in [
        (Theme::System, "Use system setting"),
        (Theme::Light, "Light theme"),
        (Theme::Dark, "Dark theme"),
    ] {
        let change = vm.clone();
        content = content.child(
            radio_button()
                .content(label)
                .automation_properties_name(label)
                .group_name("Appearance")
                .is_checked(Some(settings.theme == theme))
                .font_size(14.0)
                .min_height(40.0)
                .on_click(move |_| {
                    let mut settings = change.state().settings.clone();
                    settings.theme = theme;
                    change.dispatch(Action::SetSettings(settings));
                    change.request(HostRequest::SetTheme { theme });
                }),
        );
    }
    content = content
        .child(section_label("General", p.text))
        .child(toggle(
            vm.clone(),
            "Confirm before deleting",
            settings.confirm_delete,
            |s, v| s.confirm_delete = v,
        ))
        .child(toggle(
            vm.clone(),
            "Play completion sound",
            settings.completion_sound,
            |s, v| s.completion_sound = v,
        ))
        .child(toggle(
            vm.clone(),
            "Show completed tasks",
            settings.show_completed,
            |s, v| s.show_completed = v,
        ))
        .child(toggle(
            vm.clone(),
            "Week starts on Monday",
            settings.week_starts_monday,
            |s, v| s.week_starts_monday = v,
        ))
        .child(toggle(
            vm.clone(),
            "Recognize dates and times (English)",
            settings.recognize_dates,
            |s, v| s.recognize_dates = v,
        ))
        .child(toggle(
            vm.clone(),
            "Remove recognized dates from titles",
            settings.remove_recognized_dates,
            |s, v| s.remove_recognized_dates = v,
        ))
        .child(section_label("Smart lists", p.text))
        .child(toggle(vm.clone(), "All", settings.show_all, |s, v| {
            s.show_all = v
        }))
        .child(toggle(
            vm.clone(),
            "Completed",
            settings.show_completed_list,
            |s, v| s.show_completed_list = v,
        ))
        .child(toggle(
            vm.clone(),
            "Assigned to me",
            settings.show_assigned,
            |s, v| s.show_assigned = v,
        ))
        .child(toggle(
            vm.clone(),
            "Flagged email",
            settings.show_flagged_email,
            |s, v| s.show_flagged_email = v,
        ))
        .child(section_label("Microsoft account", p.text));
    if vm.cloud_connected() {
        let sync = vm.clone();
        let disconnect = vm.clone();
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
            .child(action_button(Icon::Person, "Disconnect", move || {
                disconnect.request(HostRequest::DisconnectMicrosoft)
            }));
    } else {
        let field = client_id_field;
        let connect = vm.clone();
        content = content.child(text_block().text("Connect using your Microsoft Entra application's public SPA client ID. The redirect URL must match this site's address.")
            .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
            .child(text_box().name("MicrosoftClientId").automation_properties_name("Application client ID")
                .text(settings.microsoft_client_id).placeholder_text("Application (client) ID").node_ref(field).font_size(14.0))
            .child(primary_button("Connect Microsoft account", move || {
                let client_id = field.get_untracked().and_then(|f| f.text()).unwrap_or_default().trim().to_string();
                if client_id.is_empty() { connect.report_error("Enter your registered application's client ID".into()); return; }
                let mut settings = connect.state().settings.clone(); settings.microsoft_client_id = client_id.clone();
                connect.dispatch(Action::SetSettings(settings)); connect.request(HostRequest::ConnectMicrosoft { client_id });
            }));
    }
    let export = vm.clone();
    let import = vm.clone();
    let notify = vm.clone();
    let help = vm.clone();
    content = content.child(section_label("Your data", p.text))
        .child(action_button(Icon::Download, "Export backup", move || export.export_backup()))
        .child(action_button(Icon::Upload, "Restore backup…", move || import.request(HostRequest::Import)))
        .child(action_button(Icon::Bell, "Enable notifications", move || notify.request(HostRequest::RequestNotifications)))
        .child(text_block().text("Reminders are delivered while this app is open. File attachments are stored in this browser; JSON backups include file references only.")
            .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
        .child(section_label("About", p.text))
        .child(action_button(Icon::Info, "Keyboard shortcuts", move || help.show_panel(Panel::Help)))
        .child(text_block().text("FerroUI To Do  ·  0.1.0\nBuilt with Rust and FerroUI").font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap).margin((0.0, 12.0)));
    border()
        .background(p.background)
        .border_brush(p.border)
        .border_thickness((1.0, 0.0, 0.0, 0.0))
        .child(
            grid()
                .row_definitions("52,*")
                .child(
                    grid()
                        .column_definitions("*,Auto")
                        .margin((20.0, 8.0, 12.0, 8.0))
                        .child(
                            text_block()
                                .name("SettingsTitle")
                                .text("Settings")
                                .font_size(18.0)
                                .font_weight(FontWeight::SemiBold)
                                .foreground(p.text)
                                .vertical_alignment(V::Center),
                        )
                        .child(border().grid_column(1).child(icon_button(
                            Icon::Close,
                            "Close settings",
                            move || close.show_panel(Panel::None),
                        ))),
                )
                .child(
                    scroll_viewer()
                        .name("SettingsScroll")
                        .grid_row(1)
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                        .child(content),
                ),
        )
        .into()
}
