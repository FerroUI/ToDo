pub(super) mod fields;
mod signals;

use crate::palette;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::TextWrapping;
use ferroui_controls::{primitives::ToggleButton, Control, TextBox};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::primary_button;
use todo_domain::{Id, RepeatDay, RepeatUnit};
use todo_state::{AppViewModel, RepeatDraft};

pub fn repeat(vm: Rc<AppViewModel>, id: Id) -> Element {
    repeat_bulk(vm, vec![id])
}

pub fn repeat_bulk(vm: Rc<AppViewModel>, ids: Vec<Id>) -> Element {
    rerender(move || {
        let p = palette(&vm);
        let initial = RepeatDraft::from_task(
            ids.first()
                .and_then(|id| vm.state().task(*id).cloned())
                .as_ref(),
            vm.today(),
            vm.state().settings.week_starts_monday,
        );
        let form = signals::Form::create(&initial);
        let error = use_signal(String::new);
        let root = use_signal(NodeRef::<StackPanel>::new).get_untracked();
        let custom = vm.clone();
        let targets = ids.clone();
        let submit: Rc<dyn Fn()> = Rc::new(move || {
            if let Err(invalid) = custom.save_bulk_repeat_draft(&targets, &form.draft()) {
                error.set(invalid.message);
                if let Some(surface) = root.get_untracked() {
                    if let Some(field) = surface
                        .find_descendant_of_type_where::<Control>(false, |control| {
                            control.name().as_deref() == Some(invalid.field)
                        })
                    {
                        if let Some(input) = field.clone().cast::<TextBox>() {
                            input.focus();
                            input.select_all();
                        } else if let Some(input) =
                            field.find_descendant_of_type_where::<TextBox>(false, |_| true)
                        {
                            input.focus();
                            input.select_all();
                        } else if let Some(day) =
                            field.find_descendant_of_type_where::<ToggleButton>(false, |_| true)
                        {
                            day.focus();
                        } else {
                            field.focus();
                        }
                    }
                }
            }
        });
        let draft = form.draft();
        let unit = draft.unit;
        let mut content = stack_panel()
            .name("RepeatForm")
            .node_ref(root)
            .spacing(12.0)
            .margin(4.0)
            .child(
                text_block()
                    .text("Choose the pattern, start and end of this schedule.")
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
                    .child(border().grid_column(1).child(fields::input(
                        "RepeatInterval",
                        "Repeat interval",
                        form.interval,
                        error,
                        submit.clone(),
                        5,
                    )))
                    .child(
                        combo_box()
                            .name("RepeatUnit")
                            .grid_column(2)
                            .min_width(0.0)
                            .automation_properties_name("Repeat unit")
                            .items_source(ferroui_controls::ItemsSource::from_strs([
                                "days", "weekdays", "weeks", "months", "years",
                            ]))
                            .selected_index(form.unit.two_way())
                            .horizontal_alignment(H::Stretch),
                    ),
            );
        if unit == RepeatUnit::Year {
            content = content.child(fields::labelled(
                &vm,
                "Month",
                combo_box()
                    .name("RepeatMonth")
                    .automation_properties_name("Repeat month")
                    .min_width(0.0)
                    .horizontal_alignment(H::Stretch)
                    .items_source(ferroui_controls::ItemsSource::from_strs([
                        "January",
                        "February",
                        "March",
                        "April",
                        "May",
                        "June",
                        "July",
                        "August",
                        "September",
                        "October",
                        "November",
                        "December",
                    ]))
                    .selected_index(form.month.two_way())
                    .into(),
            ));
        }
        if matches!(unit, RepeatUnit::Month | RepeatUnit::Year) {
            content = content.child(fields::labelled(
                &vm,
                "On",
                combo_box()
                    .name("RepeatPatternMode")
                    .automation_properties_name("Repeat date pattern")
                    .min_width(0.0)
                    .horizontal_alignment(H::Stretch)
                    .items_source(ferroui_controls::ItemsSource::from_strs([
                        "Day of the month",
                        "Weekday of the month",
                    ]))
                    .selected_index(form.relative.two_way())
                    .into(),
            ));
            if draft.relative {
                content = content.child(
                    combo_box()
                        .name("RepeatOrdinal")
                        .automation_properties_name("Week of the month")
                        .min_width(0.0)
                        .horizontal_alignment(H::Stretch)
                        .items_source(ferroui_controls::ItemsSource::from_strs([
                            "First", "Second", "Third", "Fourth", "Last",
                        ]))
                        .selected_index(form.ordinal.two_way()),
                );
            } else {
                content = content.child(fields::input(
                    "RepeatMonthDay",
                    "Day of the month",
                    form.day,
                    error,
                    submit.clone(),
                    2,
                ));
            }
        }
        if unit == RepeatUnit::Week
            || (matches!(unit, RepeatUnit::Month | RepeatUnit::Year) && draft.relative)
        {
            content = content.child(fields::labelled(
                &vm,
                "Repeat on",
                fields::days(&vm, form.days),
            ));
            if unit != RepeatUnit::Week {
                content = content.child(fields::labelled(
                    &vm,
                    "With multiple weekdays",
                    combo_box()
                        .name("RepeatRelativeMode")
                        .automation_properties_name("Relative weekday matching")
                        .min_width(0.0)
                        .horizontal_alignment(H::Stretch)
                        .items_source(ferroui_controls::ItemsSource::from_strs([
                            "First matching weekday (Microsoft To Do)",
                            "Count matching days (Outlook)",
                        ]))
                        .selected_index(form.relative_mode.two_way())
                        .into(),
                ));
                content=content.child(text_block().text("Counting matching days supports schedules such as the last weekday. Microsoft To Do sync supports one weekday or the first matching date.").font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap));
            }
        }
        if unit == RepeatUnit::Week {
            content = content.child(fields::labelled(
                &vm,
                "Week begins on",
                combo_box()
                    .name("RepeatWeekStart")
                    .automation_properties_name("Repeat week starts on")
                    .min_width(0.0)
                    .horizontal_alignment(H::Stretch)
                    .items_source(ferroui_controls::ItemsSource::from_strs(
                        RepeatDay::ALL.map(|day| day.label()),
                    ))
                    .selected_index(form.week_start.two_way())
                    .into(),
            ));
        }
        content = content
            .child(fields::labelled(
                &vm,
                "Starts",
                fields::date(&vm, "RepeatStart", "Repeat start date", form.start, error),
            ))
            .child(fields::labelled(
                &vm,
                "Ends",
                combo_box()
                    .name("RepeatEnding")
                    .automation_properties_name("Repeat ends")
                    .min_width(0.0)
                    .horizontal_alignment(H::Stretch)
                    .items_source(ferroui_controls::ItemsSource::from_strs([
                        "Never",
                        "On a date",
                        "After a number of occurrences",
                    ]))
                    .selected_index(form.ending.two_way())
                    .into(),
            ));
        match draft.ending {
            1 => {
                content = content.child(fields::date(
                    &vm,
                    "RepeatUntil",
                    "Repeat end date",
                    form.until,
                    error,
                ))
            }
            2 => {
                content = content.child(fields::labelled(
                    &vm,
                    "Total occurrences (including the first)",
                    fields::input(
                        "RepeatCount",
                        "Repeat occurrence count",
                        form.count,
                        error,
                        submit.clone(),
                        10,
                    ),
                ))
            }
            _ => {}
        }
        content.child(fields::labelled(&vm,"Time zone (optional)",fields::input("RepeatTimeZone","Repeat time zone",form.time_zone,error,submit.clone(),128)))
            .child(text_block().text("Keep blank for local calendar dates. Imported Microsoft time zone names are preserved.").font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
            .child(text_block().name("RepeatPreview").automation_properties_name("Upcoming repeat dates").text(draft.preview(vm.today())).font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
            .child(super::validation::message(error.get(),vm.is_dark()))
            .child(primary_button("Save custom repeat",move ||submit())).into_element()
    })
}
