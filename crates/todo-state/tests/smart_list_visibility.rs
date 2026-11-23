mod common;
use common::vm;
use todo_domain::{Action, View};

#[test]
fn explicitly_hiding_current_smart_list_returns_to_my_day_but_auto_hide_does_not() {
    let vm = vm();
    vm.navigate(View::Important);
    let mut settings = vm.state().settings.clone();
    settings.hide_empty_smart_lists = true;
    vm.dispatch(Action::SetSettings(settings.clone()));
    assert_eq!(vm.view(), View::Important);
    settings.show_important = false;
    vm.dispatch(Action::SetSettings(settings));
    assert_eq!(vm.view(), View::MyDay);
    vm.undo();
    assert!(vm.state().settings.show_important);
}
