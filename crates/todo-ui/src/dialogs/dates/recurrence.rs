use crate::palette;
use ferroui_base::input::Key;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::TextWrapping;
use ferroui_controls::TextBox;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::primary_button;
use todo_domain::{Id, RepeatUnit};
use todo_state::AppViewModel;

const UNITS: [RepeatUnit; 5] = [
    RepeatUnit::Day,
    RepeatUnit::Weekday,
    RepeatUnit::Week,
    RepeatUnit::Month,
    RepeatUnit::Year,
];

pub fn repeat(vm: Rc<AppViewModel>, id: Id) -> Element {
    rerender(move || {
        let p = palette(&vm);
        let existing = vm.state().task(id).and_then(|task| task.recurrence.clone());
        let interval = use_signal(|| {
            existing
                .as_ref()
                .map_or_else(|| "1".into(), |rule| rule.every.to_string())
        });
        let unit = use_signal(|| {
            existing
                .as_ref()
                .and_then(|rule| UNITS.iter().position(|candidate| *candidate == rule.unit))
                .unwrap_or(0) as i32
        });
        let error = use_signal(String::new);
        let field = use_signal(NodeRef::<TextBox>::new).get_untracked();
        let custom = vm.clone();
        let submit: Rc<dyn Fn()> = Rc::new(move || {
            let chosen = UNITS
                .get(unit.get().max(0) as usize)
                .copied()
                .unwrap_or(RepeatUnit::Day);
            if let Err(message) = custom.save_custom_repeat(id, &interval.get(), chosen) {
                error.set(message);
                if let Some(field) = field.get_untracked() {
                    field.focus();
                    field.select_all();
                }
            }
        });
        let enter = submit.clone();
        stack_panel()
            .spacing(16.0)
            .margin((4.0, 4.0))
            .child(
                text_block()
                    .text("Choose how often this task repeats.")
                    .font_size(13.0)
                    .foreground(p.secondary)
                    .text_wrapping(TextWrapping::Wrap),
            )
            .child(
                grid()
                    .column_definitions("Auto,68,*")
                    .column_spacing(8.0)
                    .child(
                        text_block()
                            .text("Every")
                            .font_size(13.0)
                            .foreground(p.text)
                            .vertical_alignment(V::Center),
                    )
                    .child(
                        text_box()
                            .name("RepeatInterval")
                            .node_ref(field)
                            .grid_column(1)
                            .automation_properties_name("Repeat interval")
                            .automation_properties_help_text("A positive whole number")
                            .text(interval.two_way())
                            .max_length(5)
                            .on_changed(TextBox::text_property(), move |_| error.set(String::new()))
                            .on_key_down(move |event| {
                                if event.key == Key::Return && !crate::editing::is_composing(event)
                                {
                                    enter();
                                    event.set_handled(true);
                                }
                            }),
                    )
                    .child(
                        combo_box()
                            .name("RepeatUnit")
                            .grid_column(2)
                            .min_width(0.0)
                            .automation_properties_name("Repeat unit")
                            .items_source(ferroui_controls::ItemsSource::from_strs([
                                "days", "weekdays", "weeks", "months", "years",
                            ]))
                            .selected_index(unit.two_way())
                            .horizontal_alignment(H::Stretch),
                    ),
            )
            .child(super::validation::message(error.get(), vm.is_dark()))
            .child(primary_button("Save custom repeat", move || submit()))
            .into_element()
    })
}
