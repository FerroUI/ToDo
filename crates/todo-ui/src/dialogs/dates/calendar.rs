use chrono::Datelike;
use ferroui_base::input::InputElement;
use ferroui_base::layout::{HorizontalAlignment, Layoutable};
use ferroui_base::utilities::{DateTime, DayOfWeek};
use ferroui_controls::Calendar;
use ferroui_view::prelude::*;
use todo_domain::Date;
use todo_state::AppViewModel;

/// Native Calendar supplies keyboard day/month/year navigation and selection peers.
pub(super) fn view(vm: &AppViewModel, selected: Signal<Date>) -> Element {
    calendar()
        .name("ScheduleCalendar")
        .focusable(true)
        .is_tab_stop(true)
        .automation_properties_name("Choose a date")
        .automation_properties_help_text("Use arrow keys to select a day. Page Up and Page Down change month. Use Tab to reach Save.")
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .first_day_of_week(if vm.state().settings.week_starts_monday { DayOfWeek::Monday } else { DayOfWeek::Sunday })
        .is_today_highlighted(true)
        .display_date(native(selected.get()))
        .selected_date(move || Some(native(selected.get())))
        .on_changed(Calendar::selected_date_property(), move |value| {
            if let Some(value) = value {
                if let Some(date) = Date::from_ymd_opt(value.year(), value.month() as u32, value.day() as u32) {
                    selected.set(date);
                }
            }
        })
        .with(|calendar| {
            // Preserve native chrome while letting seven date columns fit a compact dialog.
            calendar.styles().add(style("CalendarDayButton")
                .set(Layoutable::min_width_property(), 0.0)
                .set(InputElement::is_tab_stop_property(), false).build());
            // The native Calendar owns arrow navigation in all three modes.
            calendar.styles().add(style("CalendarButton")
                .set(InputElement::is_tab_stop_property(), false).build());
            super::calendar_accessibility::attach(calendar);
        })
        .into()
}

fn native(date: Date) -> DateTime {
    DateTime::try_new(date.year(), date.month() as i32, date.day() as i32).unwrap_or(
        if date.year() < 1 {
            DateTime::MIN_VALUE
        } else {
            DateTime::MAX_VALUE.date()
        },
    )
}
