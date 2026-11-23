mod focus;
mod group;
mod header;
mod item;
mod keys;
mod lifetime;
mod list;
mod sort;
mod task;
mod task_action;
mod task_dates;
mod task_schedule;

use ferroui_base::input::ContextRequestedEventArgs;
use ferroui_base::Ref;
use ferroui_controls::{Button, Control, Grid};
use ferroui_view::on_cleanup;
use std::rc::Rc;
use todo_domain::Id;
use todo_state::AppViewModel;

pub(crate) use item::{color_choice, color_command, command, command_detail, destructive};

/// Attach a feature's live command factory to an ordinary menu button.
pub(crate) fn attach_actions(
    button: &Ref<Button>,
    vm: Rc<AppViewModel>,
    factory: impl Fn() -> Vec<ferroui_view::Element> + 'static,
) {
    lifetime::attach(&button.clone().upcast(), vm, move |_| factory());
    open_on_click(button);
}

/// The identity reader runs on opening, after a virtualized row's tag is patched.
pub(crate) fn attach_task(
    row: &Ref<Grid>,
    vm: Rc<AppViewModel>,
    identity: fn(&Control) -> Option<Id>,
) {
    let model = vm.clone();
    lifetime::attach(&row.clone().upcast(), vm, move |anchor| {
        identity(anchor).map_or_else(Vec::new, |id| task::items(model.clone(), id, anchor))
    });
}

pub(crate) fn attach_list(button: &Ref<Button>, vm: Rc<AppViewModel>, id: Id) {
    let model = vm.clone();
    lifetime::attach(&button.clone().upcast(), vm, move |anchor| {
        let mut items = list::items(model.clone(), id);
        items.insert(
            0,
            crate::clipboard::paste_command(anchor, model.clone(), todo_domain::View::List(id)),
        );
        items
    });
}

/// Detail panes retain their toolbar while the selected task changes.
pub(crate) fn task_options(button: &Ref<Button>, vm: Rc<AppViewModel>) {
    let model = vm.clone();
    lifetime::attach(&button.clone().upcast(), vm, move |anchor| {
        model
            .selected()
            .map_or_else(Vec::new, |id| task::items(model.clone(), id, anchor))
    });
    open_on_click(button);
}

pub(crate) fn header_options(button: &Ref<Button>, vm: Rc<AppViewModel>) {
    let model = vm.clone();
    lifetime::attach(&button.clone().upcast(), vm, move |anchor| {
        let mut items = header::items(model.clone());
        items.insert(
            4,
            crate::clipboard::paste_command(anchor, model.clone(), model.view()),
        );
        items
    });
    open_on_click(button);
}

pub(crate) fn header_sort(button: &Ref<Button>, vm: Rc<AppViewModel>) {
    let model = vm.clone();
    lifetime::attach(&button.clone().upcast(), vm, move |_| {
        sort::items(model.clone())
    });
    open_on_click(button);
}

pub(crate) fn group_options(button: &Ref<Button>, vm: Rc<AppViewModel>, id: Id) {
    attach_group(button, vm, id);
    open_on_click(button);
}

pub(crate) fn selection_options(button: &Ref<Button>, vm: Rc<AppViewModel>) {
    let model = vm.clone();
    lifetime::attach(&button.clone().upcast(), vm, move |anchor| {
        model
            .ordered_selection()
            .first()
            .copied()
            .map_or_else(Vec::new, |id| task::items(model.clone(), id, anchor))
    });
    open_on_click(button);
}

pub(crate) fn attach_group(button: &Ref<Button>, vm: Rc<AppViewModel>, id: Id) {
    let model = vm.clone();
    lifetime::attach(&button.clone().upcast(), vm, move |_| {
        group::items(model.clone(), id)
    });
}

/// Use the same opening event and placement rules as a native keyboard request.
pub(crate) fn open(anchor: &Control) {
    if anchor.is_attached_to_visual_tree() && anchor.context_menu().is_some() {
        anchor.raise_event(&ContextRequestedEventArgs::new());
    }
}

fn open_on_click(button: &Ref<Button>) {
    let weak = button.downgrade();
    let token = button.add_handler(Button::click_event(), move |_, event| {
        if let Some(button) = weak.upgrade() {
            open(&button);
            event.set_handled(true);
        }
    });
    let weak = button.downgrade();
    on_cleanup(move || {
        if let Some(button) = weak.upgrade() {
            button.remove_handler(Button::click_event(), token);
        }
    });
}
