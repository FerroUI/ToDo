use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{Icon, icon};
use todo_domain::View;
use todo_state::AppViewModel;
pub(super) fn view(vm: Rc<AppViewModel>, color: &str) -> Element {
    let (symbol, title, body) = match vm.view() {
        View::MyDay => (
            Icon::Sun,
            "Focus on your day",
            "Get things done with My Day, a list that refreshes every day.",
        ),
        View::Important => (
            Icon::Star,
            "Your important tasks",
            "Star a task to find it here and keep your priorities in sight.",
        ),
        View::Planned => (
            Icon::Calendar,
            "Plan your next steps",
            "Tasks with a due date or reminder appear here.",
        ),
        View::Search(query) if query.trim().is_empty() => (
            Icon::Search,
            "Search your tasks",
            "Find a task by its name, steps, notes, or tags.",
        ),
        View::Search(_) => (
            Icon::Search,
            "No results found",
            "Try searching for a different word in your tasks, steps, or notes.",
        ),
        View::Completed => (
            Icon::Check,
            "A fresh start",
            "Your completed tasks will appear here.",
        ),
        View::AssignedToMe if vm.cloud_connected() => (
            Icon::Person,
            "No tasks assigned to you",
            "Tasks assigned to you will appear here when available to this app.",
        ),
        View::FlaggedEmail if vm.cloud_connected() => (
            Icon::Flag,
            "No flagged email",
            "Sync your account to check for flagged email tasks.",
        ),
        View::AssignedToMe | View::FlaggedEmail => (
            Icon::Person,
            "Connect your account",
            "Connect Microsoft in Settings to access your account's task lists.",
        ),
        _ => (
            Icon::List,
            "A little planning goes a long way",
            "Add your first task below.",
        ),
    };
    let my_day = matches!(vm.view(), View::MyDay);
    stack_panel()
        .vertical_alignment(V::Center)
        .horizontal_alignment(H::Center)
        .spacing(16.0)
        .max_width(310.0)
        .margin((24.0, 0.0, 24.0, 60.0))
        .child(
            border()
                .horizontal_alignment(H::Center)
                .padding(18.0)
                .corner_radius(48.0)
                .background(if my_day { "#1FFFFFFF" } else { "#112564CF" })
                .child(icon(symbol, 48.0, color)),
        )
        .child(
            text_block()
                .text(title)
                .font_size(20.0)
                .font_weight(FontWeight::SemiBold)
                .foreground(color)
                .text_alignment(ferroui_base::media::TextAlignment::Center)
                .text_wrapping(TextWrapping::Wrap),
        )
        .child(
            text_block()
                .text(body)
                .font_size(13.0)
                .line_height(21.0)
                .foreground(color)
                .opacity(0.9)
                .text_alignment(ferroui_base::media::TextAlignment::Center)
                .text_wrapping(TextWrapping::Wrap),
        )
        .into()
}
