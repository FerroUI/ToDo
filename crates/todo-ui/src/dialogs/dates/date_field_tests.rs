use super::*;
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::templates::{
    FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
};
use ferroui_controls::{Button, Calendar, ComboBox, Control, Panel, TextBlock};
use todo_domain::AppState;
use todo_state::AppViewModel;

fn template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::for_type::<CalendarDatePicker>(|control, scope| {
        let input = TextBox::new();
        input.set_name(Some("PART_TextBox".into()));
        let input = input.register_in_name_scope(&**scope);
        let calendar = Calendar::new();
        calendar.set_name(Some("PART_Calendar".into()));
        calendar.bind_indexer(
            &Calendar::selected_date_property().bind(),
            &control.indexer(&CalendarDatePicker::selected_date_property().bind()),
        );
        let calendar = calendar.register_in_name_scope(&**scope);
        let panel = Panel::new();
        panel.children().add(input);
        panel.children().add(calendar);
        panel.upcast()
    })
}

struct Field {
    owner: Owner,
    picker: Ref<CalendarDatePicker>,
    value: Signal<String>,
    error: Signal<String>,
}

impl Field {
    fn new(initial: &str) -> Self {
        let owner = Owner::new_root();
        let (picker, value, error) = owner.with(|| {
            let value = signal(initial.to_owned());
            let error = signal(String::new());
            let vm = Rc::new(AppViewModel::new(
                AppState::new(),
                "2026-11-23".parse().unwrap(),
                Rc::new(|_| Ok(())),
            ));
            let surface = rerender(move || {
                // The real recurrence form reads the draft on every render.
                value.get();
                super::super::recurrence::fields::date(
                    &vm,
                    "RepeatStart",
                    "Start date",
                    value,
                    error,
                )
            })
            .mount();
            let picker = surface
                .clone()
                .cast::<CalendarDatePicker>()
                .or_else(|| {
                    surface.find_descendant_of_type_where::<CalendarDatePicker>(false, |_| true)
                })
                .unwrap();
            (picker, value, error)
        });
        Self {
            owner,
            picker,
            value,
            error,
        }
    }

    fn apply(&self) -> Ref<TextBox> {
        self.picker.set_template(Some(template()));
        self.picker.apply_template();
        self.picker
            .get_template_descendants()
            .into_iter()
            .find_map(|control| control.cast::<TextBox>())
            .unwrap()
    }
}

impl Drop for Field {
    fn drop(&mut self) {
        self.owner.dispose();
    }
}

fn commit(input: &Control) {
    let mut event = KeyEventArgs::new();
    event.set_routed_event(Some(InputElement::key_down_event()));
    event.key = Key::Enter;
    input.raise_event(&event);
}

#[test]
fn date_field_initializes_before_template_and_preserves_invalid_draft_on_rerender() {
    let _scope = Dispatcher::unit_test_scope();
    let field = Field::new("2026-11-23");
    assert_eq!(field.picker.selected_date(), native("2026-11-23"));
    // Also cover the framework's queued SelectedDate -> Text update before
    // PART_Calendar exists, which must not go through eager Text binding.
    Dispatcher::ui_thread().run_jobs(None);
    let input = field.apply();
    assert_eq!(input.text().as_deref(), Some("2026-11-23"));
    input.set_text(Some("2026-02-31"));
    assert_eq!(field.value.get_untracked(), "2026-02-31");
    commit(&input);
    assert!(field.error.get_untracked().contains("valid calendar date"));
    assert_eq!(field.value.get_untracked(), "2026-02-31");
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(input.text().as_deref(), Some("2026-02-31"));
    assert_eq!(field.picker.selected_date(), native("2026-11-23"));

    input.set_text(Some("2026-02-28"));
    assert!(field.error.get_untracked().is_empty());
    commit(&input);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(field.value.get_untracked(), "2026-02-28");
    assert_eq!(field.picker.selected_date(), native("2026-02-28"));
}

#[test]
fn date_field_clears_optional_date_and_tracks_calendar_after_retemplate() {
    let _scope = Dispatcher::unit_test_scope();
    let field = Field::new("");
    assert_eq!(field.picker.selected_date(), None);
    let old_input = field.apply();
    old_input.set_text(Some("2028-02-29"));
    commit(&old_input);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(field.value.get_untracked(), "2028-02-29");
    old_input.clear();
    commit(&old_input);
    assert_eq!(field.value.get_untracked(), "");
    assert_eq!(field.picker.selected_date(), None);

    let input = field.apply();
    old_input.set_text(Some("2040-01-01"));
    assert_eq!(
        field.value.get_untracked(),
        "",
        "replaced template cannot write the draft"
    );
    let calendar = field
        .picker
        .get_template_descendants()
        .into_iter()
        .find_map(|control| control.cast::<Calendar>())
        .unwrap();
    calendar.set_selected_date(native("2027-01-03"));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(field.value.get_untracked(), "2027-01-03");
    assert_eq!(input.text().as_deref(), Some("2027-01-03"));
    field.picker.clear();
    assert_eq!(field.value.get_untracked(), "");
    assert_eq!(input.text().as_deref(), Some(""));

    input.set_text(Some("invalid"));
    commit(&input);
    field.owner.dispose();
    // A queued rejected-text restoration must not access disposed signals.
    Dispatcher::ui_thread().run_jobs(None);
}

#[test]
fn full_recurrence_form_keeps_responding_when_interval_changes() {
    let _scope = Dispatcher::unit_test_scope();
    let _feedback = FeedbackGuard::new();
    let vm = AppViewModel::new(
        AppState::new(),
        "2026-11-23".parse().unwrap(),
        Rc::new(|_| Ok(())),
    );
    vm.add_task("Schedule regression".into());
    let id = vm.state().tasks[0].id;
    let (surface, owner) = super::super::recurrence::repeat(vm.clone(), id).mount_root();
    let picker = surface
        .find_descendant_of_type_where::<CalendarDatePicker>(false, |_| true)
        .unwrap();
    picker.set_template(Some(template()));
    picker.apply_template();
    Dispatcher::ui_thread().run_jobs(None);
    let interval = surface
        .find_descendant_of_type_where::<TextBox>(false, |input| {
            input.name().as_deref() == Some("RepeatInterval")
        })
        .unwrap();
    let unit = combo(&surface, "RepeatUnit");
    let source = unit.items_source();
    interval.set_text(Some("0"));
    commit(&interval);
    assert!(vm.state().task(id).unwrap().recurrence.is_none());
    assert!(surface
        .find_descendant_of_type_where::<TextBlock>(false, |text| text.name().as_deref()
            == Some("ScheduleValidation"))
        .unwrap()
        .text()
        .is_some_and(|text| text.contains("positive")));
    assert_eq!(
        unit.items_source(),
        source,
        "editing and validation retain the native collection identity"
    );
    assert_eq!(unit.selected_index(), 0);
    interval.set_text(Some("3"));

    let input = picker
        .get_template_descendants()
        .into_iter()
        .find_map(|control| control.cast::<TextBox>())
        .unwrap();
    input.set_text(Some("2026-02-31"));
    commit(&input);
    Dispatcher::ui_thread().run_jobs(None);
    commit(&interval);
    assert!(vm.state().task(id).unwrap().recurrence.is_none());
    assert_eq!(
        input.text().as_deref(),
        Some("2026-02-31"),
        "full-form error rerenders cannot replace the invalid draft"
    );
    input.set_text(Some("2026-11-23"));
    commit(&input);
    Dispatcher::ui_thread().run_jobs(None);
    commit(&interval);
    assert_eq!(
        vm.state()
            .task(id)
            .unwrap()
            .recurrence
            .as_ref()
            .unwrap()
            .every,
        3
    );
    owner.dispose();
}

fn combo(surface: &Control, name: &str) -> Ref<ComboBox> {
    surface
        .find_descendant_of_type_where::<ComboBox>(false, |input| {
            input.name().as_deref() == Some(name)
        })
        .unwrap()
}

#[test]
fn full_recurrence_form_retains_conditional_dropdown_selections_while_editing() {
    let _scope = Dispatcher::unit_test_scope();
    let _feedback = FeedbackGuard::new();
    let vm = AppViewModel::new(
        AppState::new(),
        "2026-11-23".parse().unwrap(),
        Rc::new(|_| Ok(())),
    );
    vm.add_task("Conditional schedule".into());
    let id = vm.state().tasks[0].id;
    let (surface, owner) = super::super::recurrence::repeat(vm.clone(), id).mount_root();
    let unit = combo(&surface, "RepeatUnit");
    unit.set_selected_index(2); // Weekly adds the week-start dropdown.
    combo(&surface, "RepeatWeekStart").set_selected_index(1);
    unit.set_selected_index(4); // Yearly adds month and pattern dropdowns.
    combo(&surface, "RepeatMonth").set_selected_index(11);
    combo(&surface, "RepeatPatternMode").set_selected_index(1);
    combo(&surface, "RepeatOrdinal").set_selected_index(4);
    surface
        .find_descendant_of_type_where::<ToggleButton>(false, |day| {
            day.name().as_deref() == Some("RepeatDayThursday")
        })
        .unwrap()
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    combo(&surface, "RepeatRelativeMode").set_selected_index(1);
    combo(&surface, "RepeatEnding").set_selected_index(1);
    assert!(surface
        .find_descendant_of_type_where::<CalendarDatePicker>(false, |input| input.name().as_deref()
            == Some("RepeatUntil"))
        .is_some());
    combo(&surface, "RepeatEnding").set_selected_index(2);
    let count = surface
        .find_descendant_of_type_where::<TextBox>(false, |input| {
            input.name().as_deref() == Some("RepeatCount")
        })
        .unwrap();
    count.set_text(Some("0"));
    commit(&count);
    assert!(vm.state().task(id).unwrap().recurrence.is_none());
    for (name, selected) in [
        ("RepeatUnit", 4),
        ("RepeatMonth", 11),
        ("RepeatPatternMode", 1),
        ("RepeatOrdinal", 4),
        ("RepeatRelativeMode", 1),
        ("RepeatEnding", 2),
    ] {
        assert_eq!(
            combo(&surface, name).selected_index(),
            selected,
            "{name} retains selection across conditional field and validation changes"
        );
    }
    count.set_text(Some("4"));
    commit(&count);
    let state = vm.state();
    let rule = state.task(id).unwrap().recurrence.as_ref().unwrap();
    assert_eq!(rule.unit, todo_domain::RepeatUnit::Year);
    assert_eq!(rule.month, Some(12));
    assert_eq!(rule.ordinal, Some(todo_domain::WeekIndex::Last));
    assert_eq!(
        rule.relative_mode,
        todo_domain::RelativeMode::NthMatchingDay
    );
    assert_eq!(rule.end, todo_domain::RepeatEnd::After(4));
    owner.dispose();
}

struct FeedbackGuard;

impl FeedbackGuard {
    fn new() -> Self {
        ferroui_view::set_diagnostics_handler(|diagnostic| {
            assert!(
                !matches!(
                    diagnostic,
                    ferroui_view::Diagnostic::RunawayComponent { .. }
                ),
                "recurrence input must settle instead of trapping the UI thread: {diagnostic}"
            );
        });
        Self
    }
}

impl Drop for FeedbackGuard {
    fn drop(&mut self) {
        ferroui_view::clear_diagnostics_handler();
    }
}
