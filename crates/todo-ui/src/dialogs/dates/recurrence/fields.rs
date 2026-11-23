use crate::palette;
use chrono::Datelike;
use ferroui_base::input::Key;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::TextWrapping;
use ferroui_base::utilities::{DateTime, DayOfWeek};
use ferroui_controls::{CalendarDatePicker, CalendarDatePickerFormat, TextBox};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_domain::{Date, RepeatDay};
use todo_state::AppViewModel;

pub fn input(
    name: &'static str,
    label: &'static str,
    value: Signal<String>,
    error: Signal<String>,
    submit: Rc<dyn Fn()>,
    max_length: i32,
) -> Element {
    text_box()
        .name(name)
        .class("todo-no-clear")
        .automation_properties_name(label)
        .text(value.two_way())
        .max_length(max_length)
        .on_changed(TextBox::text_property(), move |_| error.set(String::new()))
        .on_key_down(move |event| {
            if event.key == Key::Return && !crate::editing::is_composing(event) {
                submit();
                event.set_handled(true);
            }
        })
        .into()
}

pub fn labelled(vm: &AppViewModel, label: &str, content: Element) -> Element {
    stack_panel()
        .spacing(5.0)
        .child(
            text_block()
                .text(label)
                .font_size(13.0)
                .foreground(palette(vm).text)
                .text_wrapping(TextWrapping::Wrap),
        )
        .child(content)
        .into()
}

pub fn date(
    vm: &AppViewModel,
    name: &'static str,
    label: &'static str,
    value: Signal<String>,
    error: Signal<String>,
) -> Element {
    let selected = value.get().parse::<Date>().unwrap_or(vm.today());
    calendar_date_picker()
        .name(name)
        .automation_properties_name(label)
        .automation_properties_help_text("Enter YYYY-MM-DD or open the calendar to choose a date.")
        .min_width(0.0)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .selected_date_format(CalendarDatePickerFormat::Custom)
        .custom_date_format_string("yyyy-MM-dd")
        .display_date(
            DateTime::try_new(
                selected.year(),
                selected.month() as i32,
                selected.day() as i32,
            )
            .unwrap_or(DateTime::MIN_VALUE),
        )
        .first_day_of_week(if vm.state().settings.week_starts_monday {
            DayOfWeek::Monday
        } else {
            DayOfWeek::Sunday
        })
        .text(value.two_way())
        .on_changed(CalendarDatePicker::text_property(), move |_| {
            error.set(String::new())
        })
        .with(move |picker| {
            let subscription = picker.date_validation_error(move |_| {
                error.set("Enter a valid calendar date in YYYY-MM-DD format.".into())
            });
            on_cleanup(move || drop(subscription));
        })
        .into()
}

pub fn days(vm: &AppViewModel, days: Signal<Vec<RepeatDay>>) -> Element {
    let order = if vm.state().settings.week_starts_monday {
        [
            RepeatDay::Monday,
            RepeatDay::Tuesday,
            RepeatDay::Wednesday,
            RepeatDay::Thursday,
            RepeatDay::Friday,
            RepeatDay::Saturday,
            RepeatDay::Sunday,
        ]
    } else {
        RepeatDay::ALL
    };
    let selected = days.get();
    let mut row = grid()
        .name("RepeatDays")
        .automation_properties_name("Repeat weekdays")
        .column_definitions("*,*,*,*,*,*,*")
        .column_spacing(2.0);
    for (index, day) in order.into_iter().enumerate() {
        row = row.child(
            toggle_button()
                .grid_column(index as i32)
                .name(format!("RepeatDay{}", day.label()))
                .content(day.short())
                .min_width(0.0)
                .min_height(38.0)
                .padding((1.0, 4.0))
                .font_size(11.0)
                .automation_properties_name(format!("Repeat on {}", day.label()))
                .tool_tip_tip(day.label())
                .is_checked(Some(selected.contains(&day)))
                .on_click(move |_| {
                    let mut current = days.get_untracked();
                    if current.contains(&day) {
                        current.retain(|candidate| *candidate != day)
                    } else {
                        current.push(day)
                    }
                    days.set(current);
                }),
        );
    }
    row.into()
}
