use ferroui_controls::ItemsSource;
use todo_domain::RepeatDay;

/// Native selection is tied to the identity of its items collection. Retain
/// these sources across draft/error rerenders: replacing them resets selection,
/// whose two-way binding would write the form and trigger another rerender.
#[derive(Clone)]
pub(super) struct Choices {
    pub units: ItemsSource,
    pub months: ItemsSource,
    pub patterns: ItemsSource,
    pub ordinals: ItemsSource,
    pub relative_modes: ItemsSource,
    pub weekdays: ItemsSource,
    pub endings: ItemsSource,
}

impl Choices {
    pub fn new() -> Self {
        Self {
            units: ItemsSource::from_strs(["days", "weekdays", "weeks", "months", "years"]),
            months: ItemsSource::from_strs([
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
            ]),
            patterns: ItemsSource::from_strs(["Day of the month", "Weekday of the month"]),
            ordinals: ItemsSource::from_strs(["First", "Second", "Third", "Fourth", "Last"]),
            relative_modes: ItemsSource::from_strs([
                "First matching weekday (Microsoft To Do)",
                "Count matching days (Outlook)",
            ]),
            weekdays: ItemsSource::from_strs(RepeatDay::ALL.map(|day| day.label())),
            endings: ItemsSource::from_strs([
                "Never",
                "On a date",
                "After a number of occurrences",
            ]),
        }
    }
}
