mod common;
use common::*;

#[test]
fn malformed_remote_palette_cannot_replace_the_last_valid_catalogue() {
    let vm = vm();
    let valid = r##"[{"id":"work","name":"Work","color":"#0078D4","color_name":"Blue","preset":"preset7"}]"##;
    vm.set_outlook_categories(valid).unwrap();
    let previous = vm.outlook_categories();
    for invalid in [
        r#"[{"id":"work","name":"Work","color":"url(x)","color_name":"Blue"}]"#,
        r##"[{"id":"work","name":"","color":"#0078D4","color_name":"Blue"}]"##,
        r#"{"categories":[]}"#,
    ] {
        assert!(vm.set_outlook_categories(invalid).is_err());
        assert_eq!(vm.outlook_categories(), previous);
    }
}

#[test]
fn service_catalogue_and_runtime_capabilities_do_not_enter_task_backups() {
    let vm = vm();
    let before = vm.export_json().unwrap();
    vm.set_outlook_categories(
        r##"[{"id":"private-id","name":"Work","color":"#0078D4","color_name":"Blue"}]"##,
    )
    .unwrap();
    vm.set_background_supported(true);
    vm.set_cloud_hint("Host runtime details");
    assert_eq!(vm.export_json().unwrap(), before);
}

#[test]
fn host_preferences_cannot_write_unrelated_account_or_sync_fields() {
    let vm = vm();
    let before = vm.export_json().unwrap();
    for key in [
        "microsoft_client_id",
        "auto_sync",
        "planner_enabled",
        "theme",
    ] {
        assert!(vm.set_integration_preference(key, false).is_err());
    }
    assert_eq!(vm.export_json().unwrap(), before);
    vm.set_integration_preference("presentation_sync_enabled", true)
        .unwrap();
    assert!(vm.state().settings.presentation_sync_enabled);
    assert!(vm.state().settings.auto_sync);
}
