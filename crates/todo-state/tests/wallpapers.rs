mod common;
use common::*;
use todo_domain::{backgrounds::*, Action, AppState, View, TASKS_LIST};

fn image() -> String {
    encode_background(&[0xff, 0xd8, 0xff, 0xc0, 0, 8, 8, 0, 32, 0, 64, 1, 0xff, 0xd9]).unwrap()
}

#[test]
fn backgrounds_follow_list_smart_view_history_backup_and_account_boundaries() {
    let vm = vm();
    vm.navigate(View::List(TASKS_LIST));
    vm.set_wallpaper(Some(image()));
    let original = vm.wallpaper();
    vm.set_smart_color("#123456".into());
    assert_eq!(
        vm.wallpaper(),
        original,
        "a color change keeps its wallpaper"
    );
    vm.set_wallpaper(Some("none".into()));
    vm.undo();
    assert_eq!(vm.wallpaper(), original);
    vm.redo();
    assert_eq!(vm.wallpaper().as_deref(), Some("none"));
    vm.navigate(View::Important);
    vm.set_wallpaper(Some("scene:forest".into()));
    let document = vm.export_json().unwrap();
    let restored = AppState::from_json(&document).unwrap();
    assert_eq!(
        restored
            .settings
            .smart_backgrounds
            .get("Important")
            .map(String::as_str),
        Some("scene:forest")
    );
    assert_eq!(
        restored.list(TASKS_LIST).unwrap().background.as_deref(),
        Some("none")
    );
    vm.replace_account_document(&AppState::new().to_json().unwrap())
        .unwrap();
    vm.navigate(View::Important);
    assert_eq!(
        vm.wallpaper(),
        None,
        "another account never inherits the previous wallpaper"
    );
    vm.replace_account_document(&document).unwrap();
    vm.navigate(View::Important);
    assert_eq!(vm.wallpaper().as_deref(), Some("scene:forest"));
}

#[test]
fn image_budget_rejects_mutations_and_duplicates_before_any_state_changes() {
    let mut bytes = vec![0; MAX_BACKGROUND_BYTES];
    bytes[..14].copy_from_slice(&[0xff, 0xd8, 0xff, 0xc0, 0, 8, 8, 0, 32, 0, 64, 1, 0xff, 0xd9]);
    let image = encode_background(&bytes).unwrap();
    let mut state = AppState::new();
    let mut last = TASKS_LIST;
    for index in 0..7 {
        last = state
            .apply(
                Action::AddList {
                    name: format!("Image {index}"),
                },
                today(),
            )
            .unwrap()
            .created_list
            .unwrap();
        state
            .apply(
                Action::SetListTheme {
                    id: last,
                    color: "#123456".into(),
                    background: Some(image.clone()),
                },
                today(),
            )
            .unwrap();
    }
    let before = state.clone();
    let error = state
        .apply(Action::DuplicateList(last), today())
        .unwrap_err();
    assert!(error.to_string().contains("2 MiB"));
    assert_eq!(
        state, before,
        "failed duplication cannot consume IDs or add entities"
    );
    assert!(state
        .apply(
            Action::SetListTheme {
                id: TASKS_LIST,
                color: "#abcdef".into(),
                background: Some(image.clone())
            },
            today()
        )
        .is_err());
    assert_eq!(state, before);
    let mut settings = state.settings.clone();
    settings
        .smart_backgrounds
        .insert("My Day".into(), image.clone());
    assert!(state
        .apply(Action::SetSettings(settings.clone()), today())
        .is_err());
    assert_eq!(state, before);
    state.settings = settings;
    assert!(AppState::from_json(&state.to_json().unwrap())
        .unwrap_err()
        .to_string()
        .contains("2 MiB"));
}
