use todo_domain::{AppState, Settings, TASKS_LIST, View};

#[test]
fn legacy_settings_preserve_default_navigation_and_new_preferences_round_trip() {
    let state=AppState::from_json(r##"{"schema_version":1,"lists":[{"id":1,"name":"Tasks"}],"settings":{"show_all":false},"next_id":2}"##).unwrap();
    assert!(state.settings.show_important && state.settings.show_planned);
    assert!(!state.settings.hide_empty_smart_lists);
    assert!(!state.settings.show_all);
    let mut changed = state;
    changed.settings.show_important = false;
    changed.settings.show_planned = false;
    changed.settings.hide_empty_smart_lists = true;
    assert_eq!(
        AppState::from_json(&changed.to_json().unwrap()).unwrap(),
        changed
    );
}

#[test]
fn auto_hide_respects_explicit_toggles_and_keeps_the_current_empty_view_reachable() {
    let mut settings = Settings::default();
    settings.hide_empty_smart_lists = true;
    assert!(!settings.smart_list_visible(&View::Important, 0, &View::MyDay));
    assert!(settings.smart_list_visible(&View::Important, 1, &View::MyDay));
    assert!(settings.smart_list_visible(&View::Important, 0, &View::Important));
    settings.show_important = false;
    assert!(!settings.smart_list_visible(&View::Important, 1, &View::Important));
    for view in [
        View::MyDay,
        View::List(TASKS_LIST),
        View::Search(String::new()),
    ] {
        assert!(settings.smart_list_visible(&view, 0, &View::MyDay));
    }
}
