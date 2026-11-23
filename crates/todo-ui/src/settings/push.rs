use super::controls::section_label;
use crate::palette;
use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{action_button, primary_button, Icon};
use todo_state::{AppViewModel, HostRequest};

pub(super) fn view(vm: Rc<AppViewModel>) -> Element {
    rerender(move || content(vm.clone()))
}

fn content(vm: Rc<AppViewModel>) -> Element {
    let state = vm.push_state();
    let service = use_signal(|| state.service_url.clone());
    let public_key = use_signal(|| state.public_key.clone());
    let enrollment = use_signal(String::new);
    let service_source = use_signal(|| state.service_url.clone());
    let key_source = use_signal(|| state.public_key.clone());
    // Host status changes must not overwrite an unfinished enrollment form.
    for (draft, source, value) in [
        (service, service_source, &state.service_url),
        (public_key, key_source, &state.public_key),
    ] {
        if source.get() != *value {
            if state.enabled || draft.get() == source.get() {
                draft.set(value.clone());
            }
            source.set(value.clone());
        }
    }
    if !state.available {
        return Element::Empty;
    }
    let p = palette(&vm);
    let mut content = stack_panel().spacing(8.0)
        .child(section_label("Browser background reminders", p.text))
        .child(text_block().text("Receive reminders after this page closes using a reminder service that you or your administrator deploys. The static website alone cannot send background pushes.")
            .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
        .child(text_block().name("PushStatus").text(state.status)
            .automation_properties_live_setting(ferroui_controls::automation::AutomationLiveSetting::Polite)
            .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap));
    if state.supported || state.enabled {
        content = content
            .child(
                text_block()
                    .text("Reminder service URL")
                    .font_size(12.0)
                    .foreground(p.text),
            )
            .child(
                text_box()
                    .name("PushServiceUrl")
                    .automation_properties_name("Reminder service URL")
                    .text(service.two_way())
                    .placeholder_text("https://reminders.example.org/")
                    .is_read_only(state.enabled)
                    .max_length(2048)
                    .font_size(14.0),
            )
            .child(
                text_block()
                    .text("Public VAPID key")
                    .font_size(12.0)
                    .foreground(p.text),
            )
            .child(
                text_box()
                    .name("PushPublicKey")
                    .automation_properties_name("Public VAPID key")
                    .text(public_key.two_way())
                    .placeholder_text("Public key from your reminder service")
                    .is_read_only(state.enabled)
                    .max_length(128)
                    .font_size(14.0),
            );
        if state.enabled {
            let refresh = vm.clone();
            let disable = vm.clone();
            content = content
                .child(action_button(
                    Icon::Repeat,
                    "Refresh background reminders",
                    move || refresh.request(HostRequest::RefreshPush),
                ))
                .child(action_button(
                    Icon::Close,
                    "Disable background reminders",
                    move || disable.request(HostRequest::DisablePush),
                ));
        } else {
            let enable = vm.clone();
            let cleanup = vm.clone();
            content = content
                .child(
                    text_block()
                        .text("Enrollment key")
                        .font_size(12.0)
                        .foreground(p.text),
                )
                .child(
                    text_box()
                        .name("PushEnrollmentKey")
                        .automation_properties_name("Reminder service enrollment key")
                        .text(enrollment.two_way())
                        .placeholder_text("Enrollment key from your administrator")
                        .password_char('•')
                        .max_length(128)
                        .font_size(14.0),
                )
                .child(primary_button("Enable background reminders", move || {
                    let enrollment_key = enrollment.get().trim().to_owned();
                    enrollment.set(String::new());
                    enable.request(HostRequest::EnablePush {
                        service_url: service.get().trim().to_owned(),
                        public_key: public_key.get().trim().to_owned(),
                        enrollment_key,
                    });
                }))
                .child(action_button(
                    Icon::Close,
                    "Clean up background subscription",
                    move || cleanup.request(HostRequest::DisablePush),
                ));
        }
        content = content.child(text_block().text("Only the active account or local workspace is scheduled. Your service receives reminder titles and times, never Microsoft sign-in tokens. Changes must reach the service before a closed app can cancel a push. Delivery also depends on the browser, network and operating system.")
            .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap));
    }
    content.into()
}
