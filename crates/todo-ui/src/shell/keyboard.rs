use super::input::{focus_later, Drafts};
use ferroui_base::input::{Key, KeyEventArgs, KeyModifiers};
use ferroui_controls::{Control, RichEditBox, TextBox};
use std::rc::Rc;
use todo_domain::{Action, View};
use todo_state::{AppViewModel, Panel};

pub(super) fn handle(root: &Control, vm: &Rc<AppViewModel>, drafts: &Drafts, event: &KeyEventArgs) {
    if crate::editing::is_composing(event) {
        return;
    }
    if event.key == Key::Escape {
        let cancelled = crate::touch::cancel_active(root) | crate::drag::cancel_active(root);
        if !cancelled && !super::navigation::dismiss_popup(root) {
            super::navigation::back(vm, drafts, false);
        }
        event.set_handled(true);
        return;
    }
    if event.key == Key::Left && event.key_modifiers.contains(KeyModifiers::ALT) {
        if super::navigation::navigate_back(root) {
            event.set_handled(true);
        }
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
        .and_then(|source| source.cast::<Control>())
        .is_some_and(|source| {
            source.get_self_and_visual_ancestors().any(|control| {
                control.cast::<TextBox>().is_some() || control.cast::<RichEditBox>().is_some()
            })
        });
    if editing
        && (event.key == Key::Delete
            || (modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::META)
                && matches!(
                    event.key,
                    Key::A | Key::C | Key::X | Key::V | Key::D | Key::T | Key::Z | Key::Y
                )))
    {
        return;
    }
    if modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::META) {
        match event.key {
            Key::C | Key::X if vm.panel() == Panel::None => {
                crate::clipboard::copy(
                    root,
                    vm.clone(),
                    vm.task_action_ids(None),
                    event.key == Key::X,
                );
            }
            Key::V if vm.panel() == Panel::None => {
                crate::clipboard::paste(root, vm.clone(), vm.view())
            }
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
            Key::L => vm.create_list_inline(),
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
                let ids = vm.task_action_ids(None);
                vm.dispatch(Action::Batch(
                    ids.into_iter()
                        .map(|id| Action::SetMyDay {
                            id,
                            day: Some(vm.today()),
                        })
                        .collect(),
                ));
            }
            Key::Z if modifiers.contains(KeyModifiers::SHIFT) => vm.redo(),
            Key::Z => vm.undo(),
            Key::Y => vm.redo(),
            Key::D1 => vm.toggle_sidebar(),
            Key::P => vm.print_list(),
            Key::R => vm.refresh(),
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
