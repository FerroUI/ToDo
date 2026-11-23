use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::reactive::Subscription;
use ferroui_base::rendering::testing::MockPlatformRenderInterface;
use ferroui_base::threading::Dispatcher;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{Button, Control, TextBlock, TextBox};
use std::{cell::RefCell, rc::Rc};
use todo_domain::AppState;
use todo_state::{AppViewModel, HostRequest, HostServices};

fn click(surface: &Control, name: &str) {
    surface
        .find_descendant_of_type_where::<Button>(false, |button| {
            AutomationProperties::get_name(button).as_deref() == Some(name)
        })
        .unwrap_or_else(|| panic!("Missing category action: {name}"))
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

fn has_text(surface: &Control, value: &str) -> bool {
    surface
        .find_descendant_of_type_where::<TextBlock>(false, |text| {
            text.text().as_deref() == Some(value)
        })
        .is_some()
}

#[test]
fn category_catalog_validates_retained_draft_on_enter_after_empty_submission() {
    let _scope = Dispatcher::unit_test_scope();
    let (render_scope, _render) = MockPlatformRenderInterface::install();
    let _render_scope = Subscription::new(render_scope);
    let mut state = AppState::new();
    state.settings.outlook_categories_enabled = true;
    state.settings.outlook_categories_write_enabled = true;
    let vm = AppViewModel::new(state, "2026-11-23".parse().unwrap(), Rc::new(|_| Ok(())));
    vm.set_outlook_categories(
        r##"[{"id":"category-review","name":"#Release, 2027","preset":"preset7","color":"#0078D4","color_name":"Blue"}]"##,
    )
    .unwrap();
    let requests = Rc::new(RefCell::new(Vec::new()));
    let received = requests.clone();
    vm.set_services(HostServices {
        request: Rc::new(move |request| received.borrow_mut().push(request)),
    });
    let (surface, owner) = super::category_catalog::view(vm).mount_root();
    click(&surface, "Manage Outlook categories");
    click(&surface, "Create Outlook category");
    assert!(has_text(
        &surface,
        "Enter a category name of 1–255 characters without line breaks."
    ));
    let field = surface
        .find_descendant_of_type_where::<TextBox>(false, |field| {
            field.name().as_deref() == Some("OutlookCategoryName")
        })
        .unwrap();
    let enter = || {
        let mut event = KeyEventArgs::new();
        event.set_routed_event(Some(InputElement::key_down_event()));
        event.key = Key::Enter;
        field.raise_event(&event);
    };
    field.set_text(Some("#Release, 2027"));
    enter();
    assert!(has_text(
        &surface,
        "An Outlook category with this name already exists."
    ));
    assert!(requests.borrow().is_empty());
    assert_eq!(field.text().as_deref(), Some("#Release, 2027"));

    field.set_text(Some("  日本語, Release  "));
    enter();
    assert!(matches!(
        requests.borrow().as_slice(),
        [HostRequest::CreateOutlookCategory { name, color }]
            if name == "日本語, Release" && color == "preset7"
    ));
    assert_eq!(field.text().as_deref(), Some("  日本語, Release  "));
    owner.dispose();
}
