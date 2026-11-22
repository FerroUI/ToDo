use crate::drag::sidebar_identity;
use ferroui_base::Ref;
use ferroui_base::input::{FocusManager, InputElement, Key, KeyEventArgs, KeyModifiers};
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_controls::{Button, StackPanel};
use ferroui_view::on_cleanup;
use std::rc::Rc;
use todo_domain::{Action, SidebarItem, TASKS_LIST};
use todo_state::{AppViewModel, RenameTarget};

pub(super) fn attach(nav: &Ref<StackPanel>, vm: Rc<AppViewModel>) {
    let weak = nav.downgrade();
    let token = nav.add_handler_with(
        InputElement::key_down_event(),
        move |_, event| {
            if let Some(nav) = weak.upgrade() {
                handle(&nav, &vm, event);
            }
        },
        RoutingStrategies::TUNNEL,
        false,
    );
    let weak = nav.downgrade();
    on_cleanup(move || {
        if let Some(nav) = weak.upgrade() {
            nav.remove_handler(InputElement::key_down_event(), token);
        }
    });
}

fn handle(nav: &StackPanel, vm: &AppViewModel, event: &KeyEventArgs) {
    if event
        .key_modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::META | KeyModifiers::ALT)
    {
        return;
    }
    let Some(source) = event.source().and_then(|source| source.cast::<Button>()) else {
        return;
    };
    if !primary(&source) {
        return;
    }
    let buttons: Vec<_> = nav
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<Button>())
        .filter(|button| primary(button) && FocusManager::can_focus(button))
        .collect();
    let Some(index) = buttons.iter().position(|button| *button == source) else {
        return;
    };
    let next = match event.key {
        Key::Up => Some(index.saturating_sub(1)),
        Key::Down => Some((index + 1).min(buttons.len() - 1)),
        Key::Home => Some(0),
        Key::End => Some(buttons.len() - 1),
        _ => None,
    };
    if let Some(next) = next {
        buttons[next].bring_into_view();
        buttons[next].focus();
        event.set_handled(true);
        return;
    }
    match (event.key, sidebar_identity(&source)) {
        (Key::Left | Key::Right, Some(SidebarItem::Group(id))) => {
            let collapsed = vm.state().group(id).map(|group| group.collapsed);
            if collapsed == Some(event.key == Key::Right) {
                vm.dispatch(Action::ToggleGroup(id));
            } else if event.key == Key::Right && index + 1 < buttons.len() {
                if sidebar_identity(&buttons[index + 1]).is_some_and(|item| matches!(item, SidebarItem::List(list) if vm.state().list(list).is_some_and(|list| list.group_id == Some(id)))) {
                        buttons[index + 1].focus();
                    }
            }
        }
        (Key::Left, Some(SidebarItem::List(id))) => {
            let group = vm.state().list(id).and_then(|list| list.group_id);
            if let Some(parent) = buttons.iter().find(|button| {
                sidebar_identity(button)
                    .is_some_and(|item| matches!(item, SidebarItem::Group(id) if Some(id) == group))
            }) {
                parent.bring_into_view();
                parent.focus();
            }
        }
        (Key::F2, Some(SidebarItem::List(id))) if id != TASKS_LIST => {
            vm.begin_rename(RenameTarget::List(id))
        }
        (Key::F2, Some(SidebarItem::Group(id))) => vm.begin_rename(RenameTarget::Group(id)),
        _ => return,
    }
    event.set_handled(true);
}

fn primary(button: &Button) -> bool {
    button.name().is_some_and(|name| {
        name.starts_with("Navigate")
            || name
                .strip_prefix("Group")
                .is_some_and(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()))
    })
}
