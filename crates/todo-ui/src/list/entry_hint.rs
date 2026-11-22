use ferroui_base::layout::VerticalAlignment;
use ferroui_view::prelude::*;
use todo_controls::{icon, Icon};
use todo_domain::TaskEntry;
use todo_state::AppViewModel;

/// Recognition is visible before submission; parsing never modifies the draft.
pub(super) fn view(vm: &AppViewModel, draft: &str) -> Element {
    if !vm.state().settings.recognize_dates {
        return Element::Empty;
    }
    let entry = TaskEntry::parse_english(draft, vm.today(), vm.current_minutes(), false);
    let (symbol, label) = if let Some(reminder) = entry.reminder {
        (
            Icon::Bell,
            format!(
                "{} {:02}:{:02}",
                vm.date_label(reminder.date),
                reminder.minutes_after_midnight / 60,
                reminder.minutes_after_midnight % 60
            ),
        )
    } else if let Some(date) = entry.due_date {
        (Icon::Calendar, vm.date_label(date))
    } else {
        return Element::Empty;
    };
    let color = crate::palette(vm).secondary;
    grid()
        .column_definitions("16,*")
        .column_spacing(5.0)
        .margin((5.0, 4.0, 8.0, 4.0))
        .vertical_alignment(VerticalAlignment::Center)
        .child(icon(symbol, 15.0, color))
        .child(
            text_block()
                .name("RecognizedSchedule")
                .grid_column(1)
                .text(label)
                .font_size(12.0)
                .text_trimming(<dyn ferroui_base::media::TextTrimming>::character_ellipsis())
                .max_lines(1)
                .foreground(color)
                .vertical_alignment(VerticalAlignment::Center),
        )
        .into()
}
