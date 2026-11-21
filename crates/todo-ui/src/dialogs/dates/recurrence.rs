use crate::palette;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{action_button, primary_button, Icon};
use todo_domain::{Action, Id, Recurrence, RepeatUnit};
use todo_state::{AppViewModel, Panel};

pub fn repeat(vm: Rc<AppViewModel>, id: Id) -> Element {
    rerender(move || {
        let p = palette(&vm);
        let mut content = stack_panel().spacing(2.0);
        for recurrence in [
            Recurrence::daily(),
            Recurrence::weekdays(),
            Recurrence::weekly(),
            Recurrence::monthly(),
            Recurrence::yearly(),
        ] {
            let choose = vm.clone();
            content = content.child(action_button(
                Icon::Repeat,
                &recurrence.label(),
                move || {
                    choose.dispatch(Action::SetRecurrence {
                        id,
                        recurrence: Some(recurrence.clone()),
                    });
                    choose.show_panel(Panel::None);
                },
            ));
        }
        let interval = use_signal(|| {
            vm.state()
                .task(id)
                .and_then(|task| task.recurrence.as_ref())
                .map(|rule| rule.every.to_string())
                .unwrap_or_else(|| "2".into())
        });
        let unit = use_signal(|| {
            vm.state()
                .task(id)
                .and_then(|task| task.recurrence.as_ref())
                .map(|rule| match rule.unit {
                    RepeatUnit::Day => 0,
                    RepeatUnit::Weekday => 1,
                    RepeatUnit::Week => 2,
                    RepeatUnit::Month => 3,
                    RepeatUnit::Year => 4,
                })
                .unwrap_or(0i32)
        });
        let custom = vm.clone();
        let remove = vm.clone();
        content
            .child(
                border()
                    .height(1.0)
                    .background(p.border)
                    .margin((8.0, 10.0)),
            )
            .child(
                text_block()
                    .text("Custom")
                    .font_size(13.0)
                    .foreground(p.secondary)
                    .margin((10.0, 4.0)),
            )
            .child(
                grid()
                    .column_definitions("Auto,60,*")
                    .column_spacing(8.0)
                    .margin(8.0)
                    .child(
                        text_block()
                            .text("Every")
                            .font_size(13.0)
                            .vertical_alignment(V::Center),
                    )
                    .child(
                        text_box()
                            .grid_column(1)
                            .text(interval.two_way())
                            .max_length(3),
                    )
                    .child(
                        combo_box()
                            .grid_column(2)
                            .items_source(ferroui_controls::ItemsSource::from_strs([
                                "days", "weekdays", "weeks", "months", "years",
                            ]))
                            .selected_index(unit.two_way())
                            .horizontal_alignment(H::Stretch),
                    ),
            )
            .child(
                border()
                    .margin(8.0)
                    .child(primary_button("Save custom repeat", move || {
                        if let Ok(every) = interval.get().parse::<u16>() {
                            if every > 0 {
                                let repeat_unit = match unit.get() {
                                    1 => RepeatUnit::Weekday,
                                    2 => RepeatUnit::Week,
                                    3 => RepeatUnit::Month,
                                    4 => RepeatUnit::Year,
                                    _ => RepeatUnit::Day,
                                };
                                custom.dispatch(Action::SetRecurrence {
                                    id,
                                    recurrence: Some(Recurrence::new(repeat_unit, every)),
                                });
                                custom.show_panel(Panel::None);
                                return;
                            }
                        }
                        custom.report_error("Repeat interval must be a positive number".into());
                    })),
            )
            .child(action_button(Icon::Close, "Never repeat", move || {
                remove.dispatch(Action::SetRecurrence {
                    id,
                    recurrence: None,
                });
                remove.show_panel(Panel::None);
            }))
            .into_element()
    })
}
