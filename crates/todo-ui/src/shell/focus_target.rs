use super::Drafts;
use crate::rows::Rows;
use ferroui_base::input::{FocusManager, InputElement, KeyboardDevice};
use ferroui_base::WeakRef;
use ferroui_controls::{automation::AutomationProperties, Control};
use todo_state::{AppViewModel, Panel};

pub(super) fn restore(
    root: &Control,
    vm: &AppViewModel,
    rows: &Rows,
    drafts: &Drafts,
    previous: Option<WeakRef<InputElement>>,
    opening_navigation: bool,
    opening_pane: bool,
    search_selection: Option<(i32, i32)>,
) {
    if opening_navigation && focus_navigation(root) {
        return;
    }
    if opening_pane && focus_pane(root, vm) {
        return;
    }
    // A still-attached editor, or a modal that already returned focus, keeps it.
    if KeyboardDevice::instance()
        .and_then(|keyboard| keyboard.focused_element())
        .is_some_and(|focused| FocusManager::can_focus(&focused))
    {
        return;
    }
    // The same search draft can move between inline navigation and its drawer.
    if let Some((start, end)) = search_selection {
        if let Some(input) = drafts
            .search_input
            .get_untracked()
            .filter(|input| FocusManager::can_focus(input))
        {
            input.focus();
            input.set_selection_start(start);
            input.set_selection_end(end);
            return;
        }
    }
    if previous
        .and_then(|previous| previous.upgrade())
        .filter(|previous| FocusManager::can_focus(previous))
        .is_some_and(|previous| previous.focus())
    {
        return;
    }
    if vm.navigation_overlay_open() && focus_navigation(root) {
        return;
    }
    if focus_pane(root, vm) {
        return;
    }
    if let Some(list) = drafts
        .task_list
        .get_untracked()
        .filter(|list| FocusManager::can_focus(list))
    {
        if let Some(index) = vm.task_cursor().and_then(|id| rows.index_of(id)) {
            list.set_selected_index(index);
            list.scroll_into_view(index);
            if list
                .container_from_index(index)
                .filter(|item| FocusManager::can_focus(item))
                .is_some_and(|item| item.focus())
            {
                return;
            }
            if list.focus() {
                return;
            }
        }
    }
    if !focus_named(root, "NewTask") {
        root.focus();
    }
}

fn focus_pane(root: &Control, vm: &AppViewModel) -> bool {
    let target = match vm.panel() {
        Panel::Settings => Some("Close settings"),
        Panel::Suggestions => Some("Close suggestions"),
        _ if vm.selected().is_some() => Some("Close details"),
        _ => None,
    };
    target.is_some_and(|name| focus_named(root, name))
}

fn focus_navigation(root: &Control) -> bool {
    // The covered list header also says "Hide navigation" while the drawer is open.
    root.find_descendant_of_type_where::<Control>(false, |control| {
        control.name().as_deref() == Some("NavigationPane")
    })
    .is_some_and(|navigation| focus_named(&navigation, "Hide navigation"))
}

fn focus_named(root: &Control, name: &str) -> bool {
    root.find_descendant_of_type_where::<Control>(false, |control| {
        (control.name().as_deref() == Some(name)
            || AutomationProperties::get_name(control).as_deref() == Some(name))
            && FocusManager::can_focus(control)
    })
    .is_some_and(|control| control.focus())
}
