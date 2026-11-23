use super::*;
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_controls::templates::{
    FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate,
};
use ferroui_controls::{Calendar, Control, Panel};
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
