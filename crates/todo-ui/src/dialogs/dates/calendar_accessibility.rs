use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::DateTime;
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::{CalendarButton, CalendarDayButton};
use ferroui_controls::{Button, Calendar, CalendarMode, Control};
use ferroui_view::on_cleanup;
use std::{cell::Cell, rc::Rc};
use todo_domain::Date;

/// Keep native keyboard/selection behavior while naming its recycled date cells.
pub(super) fn attach(calendar: &Ref<Calendar>) {
    let refresh = Rc::new(Refresh {
        calendar: calendar.downgrade(),
        pending: Cell::new(false),
        alive: Cell::new(true),
    });
    let loaded_refresh = refresh.clone();
    let loaded = calendar.loaded(move |_, _| loaded_refresh.schedule());
    let changed_refresh = refresh.clone();
    let changed = calendar.property_changed(move |event| {
        if event.property() == Calendar::display_date_property().as_property()
            || event.property() == Calendar::display_mode_property().as_property()
        {
            changed_refresh.schedule();
        }
    });
    refresh.schedule();
    on_cleanup(move || {
        refresh.alive.set(false);
        changed.dispose();
        if let Some(calendar) = refresh.calendar.upgrade() {
            calendar.remove_handler(Control::loaded_event(), loaded);
        }
    });
}

struct Refresh {
    calendar: WeakRef<Calendar>,
    pending: Cell<bool>,
    alive: Cell<bool>,
}

impl Refresh {
    fn schedule(self: &Rc<Self>) {
        if !self.alive.get() || self.pending.replace(true) {
            return;
        }
        let this = self.clone();
        Dispatcher::ui_thread().post_local(
            move || {
                this.pending.set(false);
                if let Some(calendar) = this
                    .calendar
                    .upgrade()
                    .filter(|calendar| this.alive.get() && calendar.is_attached_to_visual_tree())
                {
                    update(&calendar);
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }
}

fn update(calendar: &Calendar) {
    let period = match calendar.display_mode() {
        CalendarMode::Month => "month",
        CalendarMode::Year => "year",
        CalendarMode::Decade => "decade",
    };
    for visual in calendar.get_visual_descendants() {
        let Some(button) = visual.cast::<Button>() else {
            continue;
        };
        let label = match button.name().as_deref() {
            Some("PART_PreviousButton") => Some(format!("Previous {period}")),
            Some("PART_NextButton") => Some(format!("Next {period}")),
            Some("PART_HeaderButton") => {
                let caption = button
                    .content()
                    .and_then(|value| value.downcast_ref::<String>().cloned())
                    .unwrap_or_default();
                Some(format!("{caption}, choose month and year"))
            }
            _ => button.data_context().and_then(|value| {
                let value = value.downcast_ref::<DateTime>()?;
                let date =
                    Date::from_ymd_opt(value.year(), value.month() as u32, value.day() as u32)?;
                if button.is::<CalendarDayButton>() {
                    button.set_name(Some(format!("ScheduleDate{date}")));
                    Some(date.format("%A, %-d %B %Y").to_string())
                } else if button.is::<CalendarButton>() {
                    Some(
                        date.format(if calendar.display_mode() == CalendarMode::Year {
                            "%B %Y"
                        } else {
                            "%Y"
                        })
                        .to_string(),
                    )
                } else {
                    None
                }
            }),
        };
        if let Some(label) = label {
            AutomationProperties::set_name(&button, Some(&label));
        }
    }
}
