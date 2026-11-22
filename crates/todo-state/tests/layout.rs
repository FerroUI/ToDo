mod common;
use common::*;
use todo_domain::{View, TASKS_LIST};
use todo_state::{PaneLayout, Panel, DETAIL_WIDTH, MINIMUM_LIST_WIDTH, NAVIGATION_WIDTH};

#[test]
fn pane_budgets_fit_phone_tablet_and_desktop_widths() {
    for width in [320.0, 390.0, 760.0, 800.0, 980.0, 1024.0] {
        for right in [false, true] {
            for preferred in [false, true] {
                for open in [false, true] {
                    let layout = PaneLayout::resolve(width, preferred, open, right);
                    assert_eq!(layout.right_visible, right);
                    assert!(!(layout.navigation_inline && layout.navigation_overlay));
                    if layout.compact {
                        assert_eq!(layout.list_visible, !right);
                        assert!(!layout.navigation_inline);
                    } else {
                        assert!(layout.list_visible);
                        let used = if layout.navigation_inline {
                            NAVIGATION_WIDTH
                        } else {
                            0.0
                        } + if right { DETAIL_WIDTH } else { 0.0 };
                        assert!(
                            width - used >= MINIMUM_LIST_WIDTH,
                            "width={width}, {layout:?}"
                        );
                    }
                }
            }
        }
    }
    assert!(!PaneLayout::resolve(979.0, true, false, true).navigation_inline);
    assert!(PaneLayout::resolve(980.0, true, false, true).navigation_inline);
}

#[test]
fn settings_and_suggestions_use_the_same_right_pane_budget_as_details() {
    let vm = vm();
    vm.add_task("Task".into());
    let id = vm.state().tasks[0].id;
    for width in [320.0, 390.0, 760.0, 800.0, 980.0, 1024.0] {
        vm.set_viewport(width);
        vm.select(Some(id));
        let detail = vm.layout();
        for panel in [Panel::Settings, Panel::Suggestions] {
            vm.select(None);
            vm.show_panel(panel);
            assert_eq!(vm.layout(), detail);
        }
    }
}

#[test]
fn resizing_preserves_the_manual_navigation_preference() {
    let vm = vm();
    vm.toggle_sidebar();
    assert!(!vm.sidebar_preferred());
    for width in [390.0, 760.0, 1024.0] {
        vm.set_viewport(width);
        assert!(!vm.sidebar_visible());
        assert!(!vm.sidebar_preferred());
    }
    vm.toggle_sidebar();
    vm.show_panel(Panel::Settings);
    vm.set_viewport(800.0);
    assert!(vm.sidebar_preferred());
    assert!(!vm.sidebar_visible());
    vm.toggle_sidebar();
    assert!(vm.navigation_overlay_open());
    vm.set_viewport(980.0);
    assert!(vm.layout().navigation_inline);
    assert!(!vm.navigation_overlay_open());
}

#[test]
fn drawer_navigation_does_not_change_the_desktop_preference() {
    let vm = vm();
    vm.set_viewport(390.0);
    vm.toggle_sidebar();
    assert!(vm.navigation_overlay_open());
    vm.navigate(View::List(TASKS_LIST));
    assert!(!vm.navigation_overlay_open());
    assert!(vm.sidebar_preferred());
    vm.set_viewport(1024.0);
    assert!(vm.layout().navigation_inline);
}

#[test]
fn resize_pixels_within_one_topology_do_not_rebuild_the_view() {
    let vm = vm();
    vm.show_panel(Panel::Settings);
    vm.set_viewport(800.0);
    let revision = vm.revision();
    vm.set_viewport(810.0);
    vm.set_viewport(979.0);
    assert_eq!(vm.revision(), revision);
    vm.set_viewport(980.0);
    assert_eq!(vm.revision(), revision + 1);
    let width = vm.viewport();
    for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        vm.set_viewport(invalid);
        assert_eq!(vm.viewport(), width);
    }
}

#[test]
fn an_unsubmitted_step_survives_pane_changes_without_becoming_task_data() {
    let vm = vm();
    vm.add_task("Task".into());
    let id = vm.state().tasks[0].id;
    vm.select(Some(id));
    vm.set_step_draft(id, "Keep this unfinished step".into());
    vm.show_panel(Panel::Settings);
    vm.set_viewport(390.0);
    vm.show_panel(Panel::None);
    vm.set_viewport(1024.0);
    assert_eq!(vm.step_draft(id), "Keep this unfinished step");
    assert!(vm.state().task(id).unwrap().steps.is_empty());
    assert!(!vm.export_json().unwrap().contains("unfinished step"));
    vm.set_step_draft(id, String::new());
    assert!(vm.step_draft(id).is_empty());
}
