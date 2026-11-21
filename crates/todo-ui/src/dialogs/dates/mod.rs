mod calendar;
mod recurrence;
mod reminder;

use crate::palette;
use chrono::{Datelike, Duration, Months};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
pub use recurrence::repeat;
use reminder::apply_date;
use std::rc::Rc;
use todo_controls::{action_button, icon_button, primary_button, Icon};
use todo_domain::{Action, Id};
use todo_state::{AppViewModel, Panel};

pub fn date_picker(vm: Rc<AppViewModel>, id: Id, reminder: bool) -> Element {
    rerender(move || {
        let p = palette(&vm);
        let existing = vm.state().task(id).and_then(|task| {
            if reminder {
                task.reminder.as_ref().map(|r| r.date)
            } else {
                task.due_date
            }
        });
        let later = vm.current_minutes() + 60;
        let later_date = vm
            .today()
            .checked_add_signed(Duration::days((later / 1440) as i64))
            .unwrap_or(vm.today());
        let later_time = format!("{:02}:{:02}", (later % 1440) / 60, later % 60);
        let selected =
            use_signal(|| existing.unwrap_or(if reminder { later_date } else { vm.today() }));
        let month = use_signal(|| selected.get().with_day(1).unwrap());
        let time = use_signal(|| {
            vm.state()
                .task(id)
                .and_then(|task| task.reminder.as_ref())
                .map(|r| {
                    format!(
                        "{:02}:{:02}",
                        r.minutes_after_midnight / 60,
                        r.minutes_after_midnight % 60
                    )
                })
                .unwrap_or_else(|| later_time.clone())
        });
        let mut content = stack_panel().spacing(2.0);
        let first_label = if !reminder {
            "Today"
        } else if later < 1440 {
            "Later today"
        } else {
            "In one hour"
        };
        for (label, delta) in [(first_label, 0), ("Tomorrow", 1), ("Next week", 7)] {
            let choose = vm.clone();
            let quick_time = if reminder && delta == 0 {
                later_time.clone()
            } else {
                "09:00".into()
            };
            let delta = if reminder && delta == 0 {
                (later / 1440) as i64
            } else {
                delta
            };
            content = content.child(action_button(Icon::Calendar, label, move || {
                if let Some(date) = choose.today().checked_add_signed(Duration::days(delta)) {
                    apply_date(&choose, id, date, reminder, &quick_time);
                }
            }));
        }
        content = content
            .child(border().height(1.0).background(p.border).margin((8.0, 8.0)))
            .child(
                grid()
                    .column_definitions("*,Auto,Auto")
                    .margin((10.0, 8.0))
                    .child(
                        text_block()
                            .text(month.get().format("%B %Y").to_string())
                            .font_size(15.0)
                            .font_weight(FontWeight::SemiBold)
                            .foreground(p.text)
                            .vertical_alignment(V::Center),
                    )
                    .child(border().grid_column(1).child(icon_button(
                        Icon::ChevronLeft,
                        "Previous month",
                        move || {
                            if let Some(date) = month.get().checked_sub_months(Months::new(1)) {
                                month.set(date);
                            }
                        },
                    )))
                    .child(border().grid_column(2).child(icon_button(
                        Icon::ChevronRight,
                        "Next month",
                        move || {
                            if let Some(date) = month.get().checked_add_months(Months::new(1)) {
                                month.set(date);
                            }
                        },
                    ))),
            );
        content = content.child(calendar::view(&vm, month, selected));
        if reminder {
            content = content.child(
                grid()
                    .column_definitions("*,120")
                    .margin((8.0, 8.0))
                    .child(
                        text_block()
                            .text("Time")
                            .font_size(14.0)
                            .foreground(p.text)
                            .vertical_alignment(V::Center),
                    )
                    .child(
                        text_box()
                            .name("ReminderTime")
                            .grid_column(1)
                            .text(time.two_way())
                            .placeholder_text("HH:MM"),
                    ),
            );
        }
        let save = vm.clone();
        let remove = vm.clone();
        content = content.child(
            border()
                .horizontal_alignment(H::Stretch)
                .margin((8.0, 8.0))
                .child(primary_button("Save", move || {
                    apply_date(&save, id, selected.get(), reminder, &time.get())
                })),
        );
        if existing.is_some() {
            content = content.child(action_button(
                Icon::Close,
                if reminder {
                    "Remove reminder"
                } else {
                    "Remove due date"
                },
                move || {
                    remove.dispatch(if reminder {
                        Action::SetReminder { id, reminder: None }
                    } else {
                        Action::SetDueDate { id, date: None }
                    });
                    remove.show_panel(Panel::None);
                },
            ));
        }
        content.into_element()
    })
}
