use super::input::{focus_later, Drafts};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::{RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{Grid, TextBox, TopLevel};
use ferroui_view::on_cleanup;
use std::{cell::RefCell, rc::Rc};
use todo_domain::{Action, View};
use todo_state::{AppViewModel, HostRequest, Panel};

/// TopLevel is the keyboard target when recycling/removal clears child focus.
/// One bubbling route covers that case and normal input without stealing keys
/// already handled by a TextBox, ListBox or child button.
pub(super) fn attach(root: &Ref<Grid>, vm: Rc<AppViewModel>, drafts: Rc<Drafts>) {
    let route = Rc::new(TopLevelRoute::default());
    let weak_root = root.downgrade();
    let weak_vm = Rc::downgrade(&vm);
    let weak_drafts = Rc::downgrade(&drafts);
    let connected = route.clone();
    let attached = root.attached_to_visual_tree(move |_| {
        if let (Some(root), Some(vm), Some(drafts)) = (
            weak_root.upgrade(),
            weak_vm.upgrade(),
            weak_drafts.upgrade(),
        ) {
            connected.connect(&root, &vm, &drafts);
        }
    });
    let disconnected = route.clone();
    let detached = root.detached_from_visual_tree(move |_| disconnected.clear());
    // The mount hook normally runs before attachment; this also supports an
    // already attached root without waiting for another lifecycle event.
    route.connect(root, &vm, &drafts);
    on_cleanup(move || {
        attached.dispose();
        detached.dispose();
        route.clear();
    });
}

#[derive(Default)]
struct TopLevelRoute(RefCell<Option<(WeakRef<TopLevel>, RoutedEventHandlerToken)>>);

impl TopLevelRoute {
    fn connect(&self, root: &Ref<Grid>, vm: &Rc<AppViewModel>, drafts: &Rc<Drafts>) {
        self.clear();
        if !root.is_attached_to_visual_tree() {
            return;
        }
        let Some(top_level) = TopLevel::get_top_level(Some(root)) else {
            return;
        };
        let root = root.downgrade();
        let vm = Rc::downgrade(vm);
        let drafts = Rc::downgrade(drafts);
        let token = top_level.add_handler_with(
            InputElement::key_down_event(),
            move |_, event| {
                let Some(root) = root.upgrade().filter(|root| {
                    root.is_attached_to_visual_tree() && root.is_effectively_visible()
                }) else {
                    return;
                };
                if let (Some(vm), Some(drafts)) = (vm.upgrade(), drafts.upgrade()) {
                    handle(&vm, &drafts, event);
                }
                drop(root);
            },
            RoutingStrategies::BUBBLE,
            false,
        );
        *self.0.borrow_mut() = Some((top_level.downgrade(), token));
    }

    fn clear(&self) {
        let previous = self.0.borrow_mut().take();
        if let Some((top_level, token)) = previous {
            if let Some(top_level) = top_level.upgrade() {
                top_level.remove_handler(InputElement::key_down_event(), token);
            }
        }
    }
}

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
