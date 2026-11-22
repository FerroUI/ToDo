use super::controls::section_label;
use crate::palette;
use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{action_button, Icon};
use todo_state::{AppViewModel, HostRequest};

pub(super) fn view(vm: Rc<AppViewModel>) -> Element {
    if !vm.cloud_connected() {
        return Element::Empty;
    }
    let p = palette(&vm);
    let settings = vm.state().settings.clone();
    let categories = vm.clone();
    let refresh = vm.clone();
    let presentation = vm.clone();
    let manage = vm.clone();
    let mut content = stack_panel().spacing(8.0)
        .child(section_label("Outlook categories", p.text))
        .child(text_block().text("Choose task categories from your Outlook catalogue and show their colors. Connecting requests permission to read your mailbox settings.")
            .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
        .child(action_button(Icon::List,
            if settings.outlook_categories_enabled { "Disconnect Outlook categories" } else { "Connect Outlook categories" },
            move || categories.request(if settings.outlook_categories_enabled {
                HostRequest::DisableOutlookCategories
            } else { HostRequest::EnableOutlookCategories })));
    if settings.outlook_categories_enabled {
        content = content.child(action_button(
            Icon::Repeat,
            "Refresh category catalogue",
            move || refresh.request(HostRequest::RefreshOutlookCategories),
        ));
    }
    content.child(section_label("Across your devices", p.text))
        .child(text_block().text("Sync My Day, list groups, ordering, and appearance between installations of this app using its OneDrive app folder. Microsoft's To Do app keeps its own presentation settings.")
            .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
        .child(action_button(Icon::Repeat,
            if settings.presentation_sync_enabled { "Turn off device settings sync" } else { "Sync device settings with OneDrive" },
            move || presentation.request(if settings.presentation_sync_enabled {
                HostRequest::DisablePresentationSync
            } else { HostRequest::EnablePresentationSync })))
        .child(section_label("List collaboration", p.text))
        .child(text_block().text("Manage shared-list invitations and membership in Microsoft To Do. Shared lists available to your account are included in task sync.")
            .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
        .child(action_button(Icon::Person, "Open Microsoft To Do", move || manage.request(HostRequest::OpenMicrosoftToDo)))
        .into()
}
