use crate::rows::Rows;
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{Ref, Visual};
use ferroui_controls::{Button, Control, ListBox, ListBoxItem};
use ferroui_view::on_cleanup;
use std::rc::Rc;
use todo_domain::{Action, Id};
use todo_state::{AppViewModel, Panel};

pub(super) fn attach(list: &Ref<ListBox>, vm: Rc<AppViewModel>, rows: Rc<Rows>) {
    let list_weak = list.downgrade();
    let key_vm = vm.clone();
    let key_rows = rows.clone();
    let token = list.add_handler_with(
        InputElement::key_down_event(),
        move |_, event| {
            if let Some(list) = list_weak.upgrade() {
                handle(&list, &key_vm, &key_rows, event);
            }
        },
        RoutingStrategies::TUNNEL,
        false,
    );
    let weak = list.downgrade();
    let released = list.add_handler_with(
        InputElement::key_up_event(),
        move |_, event| {
            if (event.key == Key::Apps
                || (event.key == Key::F10 && event.key_modifiers.contains(KeyModifiers::SHIFT)))
                && matches!(
                    vm.panel(),
                    Panel::None | Panel::Suggestions | Panel::Settings
                )
            {
                let id = source_task(event)
                    .or(vm.task_cursor())
                    .or(vm.selected())
                    .or_else(|| rows.task_ids().first().copied());
                if let (Some(list), Some(id)) = (weak.upgrade(), id) {
                    open_context(&list, &rows, id);
                    event.set_handled(true);
                }
            }
        },
        RoutingStrategies::TUNNEL,
        false,
    );
    let weak = list.downgrade();
    on_cleanup(move || {
        if let Some(list) = weak.upgrade() {
            list.remove_handler(InputElement::key_down_event(), token);
            list.remove_handler(InputElement::key_up_event(), released);
        }
    });
}

fn handle(list: &Ref<ListBox>, vm: &AppViewModel, rows: &Rows, event: &KeyEventArgs) {
    if !matches!(
        vm.panel(),
        Panel::None | Panel::Suggestions | Panel::Settings
    ) {
        return;
    }
    // Let the focused button own the entire activation gesture. Intercepting
    // Space here would suppress its key-down while still allowing Button's
    // key-up to click; Return would invoke the row instead of its child action.
    // Test the actual source, not an ancestor: container focus still enables
    // the row's completion and selection shortcuts below.
    if matches!(event.key, Key::Space | Key::Return)
        && event
            .source()
            .and_then(|source| source.cast::<Button>())
            .is_some_and(|button| button.is_focused())
    {
        return;
    }
    let ids = rows.task_ids();
    if ids.is_empty() {
        return;
    }
    let control = event
        .key_modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::META);
    let shift = event.key_modifiers.contains(KeyModifiers::SHIFT);
    let cursor = source_task(event).or(vm.task_cursor()).or(vm.selected());
    let index = cursor.and_then(|id| ids.iter().position(|candidate| *candidate == id));
    let next = match event.key {
        Key::Down => Some(index.map_or(0, |index| (index + 1).min(ids.len() - 1))),
        Key::Up => Some(index.map_or(ids.len() - 1, |index| index.saturating_sub(1))),
        Key::Home => Some(0),
        Key::End => Some(ids.len() - 1),
        Key::PageDown => Some(index.unwrap_or(0).saturating_add(8).min(ids.len() - 1)),
        Key::PageUp => Some(index.unwrap_or(0).saturating_sub(8)),
        _ => None,
    };
    if let Some(next) = next {
        let id = ids[next];
        if shift {
            if !vm.selection_mode() {
                vm.set_task_cursor(cursor);
            }
            vm.select_task_range(&ids, id);
        } else {
            vm.set_task_cursor(Some(id));
            if vm.selected().is_some() && !vm.selection_mode() && !vm.is_narrow() {
                vm.select(Some(id));
            }
        }
        reveal(list, rows, id);
    } else if control && event.key == Key::A {
        vm.select_all_tasks(&ids);
    } else if let Some(id) = cursor.or_else(|| ids.first().copied()) {
        match event.key {
            Key::Space if control || vm.selection_mode() => vm.toggle_task_selection(id),
            Key::Space => vm.dispatch(Action::ToggleCompleted(id)),
            Key::Return if vm.selection_mode() => vm.toggle_task_selection(id),
            Key::Return => vm.select(Some(id)),
            // Native menu gesture opens on release, after the full key-down route.
            Key::Apps => {}
            Key::F10 if shift => {}
            Key::Delete if vm.selection_count() > 0 => vm.delete_selection(),
            Key::Delete => vm.delete_task(id),
            _ => return,
        }
    } else {
        return;
    }
    event.set_handled(true);
}

fn open_context(list: &Ref<ListBox>, rows: &Rows, id: Id) {
    let Some(index) = rows.index_of(id) else {
        return;
    };
    list.scroll_into_view(index);
    if let Some(anchor) = menu_anchor(list, index, id) {
        crate::menus::open(&anchor);
        return;
    }
    // A virtualized item is realized during the following layout pass.
    let weak = list.downgrade();
    Dispatcher::ui_thread().post_local(
        move || {
            if let Some(anchor) = weak
                .upgrade()
                .and_then(|list| menu_anchor(&list, index, id))
            {
                crate::menus::open(&anchor);
            }
        },
        DispatcherPriority::BACKGROUND,
    );
}

fn menu_anchor(list: &ListBox, index: i32, id: Id) -> Option<Ref<Control>> {
    list.container_from_index(index)?
        .find_descendant_of_type_where::<Control>(false, |control| {
            control.context_menu().is_some() && super::interactions::task_id(control) == Some(id)
        })
}

fn source_task(event: &KeyEventArgs) -> Option<Id> {
    let mut visual = event.source().and_then(|source| source.cast::<Visual>());
    while let Some(control) = visual {
        if let Some(id) = control
            .cast::<Control>()
            .and_then(|control| super::interactions::task_id(&control))
        {
            return Some(id);
        }
        // ListBox focuses its recycled container during keyboard navigation.
        // Its task identity belongs to the realized child, not the container.
        if let Some(row) = control.cast::<ListBoxItem>().and_then(|container| {
            container.find_descendant_of_type_where::<Control>(false, |child| {
                super::interactions::task_id(child).is_some()
            })
        }) {
            return super::interactions::task_id(&row);
        }
        visual = control.visual_parent();
    }
    None
}

fn reveal(list: &Ref<ListBox>, rows: &Rows, id: Id) {
    let Some(index) = rows.index_of(id) else {
        return;
    };
    list.set_selected_index(index);
    list.scroll_into_view(index);
    let weak = list.downgrade();
    Dispatcher::ui_thread().post_local(
        move || {
            if let Some(container) = weak
                .upgrade()
                .and_then(|list| list.container_from_index(index))
            {
                container.focus();
            }
        },
        DispatcherPriority::BACKGROUND,
    );
}
