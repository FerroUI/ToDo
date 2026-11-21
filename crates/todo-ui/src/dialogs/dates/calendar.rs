use crate::palette;
use chrono::{Datelike, Duration};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use todo_domain::Date;
use todo_state::AppViewModel;

pub(super) fn view(vm: &AppViewModel, month: Signal<Date>, selected: Signal<Date>) -> Element {
    let p = palette(vm);
    let monday = vm.state().settings.week_starts_monday;
    let first = month.get();
    let offset = if monday {
        first.weekday().num_days_from_monday()
    } else {
        first.weekday().num_days_from_sunday()
    };
    let start = first
        .checked_sub_signed(Duration::days(offset.into()))
        .unwrap_or(first);
    let mut calendar = grid()
        .column_definitions("*,*,*,*,*,*,*")
        .row_definitions("28,36,36,36,36,36,36")
        .margin((6.0, 2.0));
    let weekdays = if monday {
        ["M", "T", "W", "T", "F", "S", "S"]
    } else {
        ["S", "M", "T", "W", "T", "F", "S"]
    };
    for (column, label) in weekdays.into_iter().enumerate() {
        calendar = calendar.child(
            text_block()
                .grid_column(column as i32)
                .text(label)
                .font_size(11.0)
                .foreground(p.secondary)
                .horizontal_alignment(H::Center)
                .vertical_alignment(V::Center),
        );
    }
    for index in 0..42 {
        let Some(date) = start.checked_add_signed(Duration::days(index)) else {
            continue;
        };
        let is_selected = selected.get() == date;
        calendar = calendar.child(
            button()
                .class("todo-flat")
                .grid_column((index % 7) as i32)
                .grid_row((index / 7 + 1) as i32)
                .margin(1.0)
                .padding(2.0)
                .background(if is_selected { p.accent } else { "Transparent" })
                .foreground(if is_selected {
                    "#FFFFFF"
                } else if date.month() == first.month() {
                    p.text
                } else {
                    p.secondary
                })
                .opacity(if date.month() == first.month() {
                    1.0
                } else {
                    0.45
                })
                .content(date.day().to_string())
                .on_click(move |_| selected.set(date)),
        );
    }
    calendar.into()
}
