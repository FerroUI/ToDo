use super::{action_row::action, fields::section};
use crate::palette;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::Task;
use todo_state::{AppViewModel, HostRequest};

pub(super) fn view(vm: Rc<AppViewModel>, task: &Task) -> Element {
    let p = palette(&vm);
    let mut rows = stack_panel();
    let mut count = 0;
    if let Some(source) = task
        .external
        .as_ref()
        .filter(|source| valid_url(&source.url))
    {
        let label = if source.url.trim_end_matches('/') == "https://portal.apps.mil" {
            "Open Microsoft 365"
        } else {
            "Open in Planner"
        };
        rows = rows.child(open(vm.clone(), label, label, &source.url, p.accent));
        count += 1;
    }
    for source in task
        .linked_resources
        .iter()
        .filter(|source| valid_url(&source.web_url))
    {
        let label =
            if task.flagged_email && source.application_name.to_lowercase().contains("outlook") {
                "Open in Outlook".to_owned()
            } else if !source.application_name.trim().is_empty() {
                format!("Open in {}", source.application_name)
            } else if !source.display_name.trim().is_empty() {
                source.display_name.clone()
            } else {
                "Open linked item".to_owned()
            };
        let accessible = if source.display_name.trim().is_empty() {
            label.clone()
        } else {
            format!("{label}: {}", source.display_name)
        };
        rows = rows.child(open(
            vm.clone(),
            &label,
            &accessible,
            &source.web_url,
            p.accent,
        ));
        count += 1;
    }
    if count == 0 {
        Element::Empty
    } else {
        section(vm, rows)
    }
}

fn open(vm: Rc<AppViewModel>, label: &str, accessible: &str, url: &str, color: &str) -> Element {
    let url = url.to_owned();
    action(Icon::Link, label, color)
        .automation_properties_name(accessible)
        .tool_tip_tip(accessible)
        .on_click(move |event| {
            vm.request(HostRequest::OpenUrl { url: url.clone() });
            event.set_handled(true);
        })
        .into()
}

fn valid_url(url: &str) -> bool {
    (url.starts_with("https://") || url.starts_with("http://"))
        && !url.chars().any(char::is_control)
}
