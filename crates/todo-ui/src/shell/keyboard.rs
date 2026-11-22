use super::input::{focus_later, Drafts};
use ferroui_base::input::{Key, KeyEventArgs, KeyModifiers};
use ferroui_controls::TextBox;
use todo_domain::{Action, View};
use todo_state::{AppViewModel, HostRequest, Panel};

pub(super) fn handle(vm: &AppViewModel, drafts: &Drafts, event: &KeyEventArgs) {
    if event.key == Key::Escape {
        if vm.panel() != Panel::None {
            vm.show_panel(Panel::None);
        } else if vm.selected().is_some() {
            vm.select(None);
        } else if vm.selection_mode() {
            vm.end_selection();
        } else if vm.is_narrow() && vm.sidebar_visible() {
            vm.toggle_sidebar();
        }
        event.set_handled(true);
        return;
    }
    if !matches!(
        vm.panel(),
        Panel::None | Panel::Settings | Panel::Suggestions
    ) {
        return;
    }
    let modifiers = event.key_modifiers;
    let editing = event
        .source()
        .is_some_and(|source| source.cast::<TextBox>().is_some());
    if editing
        && (event.key == Key::Delete
            || (modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::META)
                && matches!(event.key, Key::D | Key::T | Key::Z | Key::Y)))
    {
        return;
    }
    if modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::META) {
        match event.key {
            Key::N => {
                if vm.selection_mode() {
                    vm.end_selection();
                }
                vm.select(None);
                vm.show_panel(Panel::None);
                if vm.is_narrow() && vm.sidebar_visible() {
                    vm.toggle_sidebar();
                }
                focus_later(drafts.task_input, false);
            }
            Key::L => vm.show_panel(Panel::NewList),
            Key::F => {
                vm.navigate(View::Search(drafts.search.get_untracked()));
                if !vm.sidebar_visible() {
                    vm.toggle_sidebar();
                }
                focus_later(drafts.search_input, true);
            }
            Key::A if !editing => {
                vm.select_all_tasks(&vm.visible_task_ids());
                if let Some(list) = drafts.task_list.get_untracked() {
                    list.focus();
                }
            }
            Key::D => {
                if vm.selection_count() > 0 {
                    vm.complete_selected_tasks();
                } else if let Some(id) = vm.selected().or(vm.task_cursor()) {
                    vm.dispatch(Action::ToggleCompleted(id));
                }
            }
            Key::T => {
                if let Some(id) = vm.selected().or(vm.task_cursor()) {
                    vm.dispatch(Action::SetMyDay {
                        id,
                        day: Some(vm.today()),
                    });
                }
            }
            Key::Z if modifiers.contains(KeyModifiers::SHIFT) => vm.redo(),
            Key::Z => vm.undo(),
            Key::Y => vm.redo(),
            Key::D1 => vm.toggle_sidebar(),
            Key::P => vm.print_list(),
            Key::R if vm.cloud_connected() => vm.request(HostRequest::SyncMicrosoft),
            _ => return,
        }
        event.set_handled(true);
    } else if event.key == Key::Delete {
        if vm.selection_count() > 0 {
            vm.delete_selection();
            event.set_handled(true);
        } else if let Some(id) = vm.selected() {
            vm.delete_task(id);
            event.set_handled(true);
        }
    }
}
