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

use ferroui_base::input::ContextRequestedEventArgs;
use ferroui_base::Ref;
use ferroui_controls::{Button, Control, Grid};
use ferroui_view::on_cleanup;
use std::rc::Rc;
use todo_domain::Id;
use todo_state::AppViewModel;

/// The identity reader runs on opening, after a virtualized row's tag is patched.
pub(crate) fn attach_task(
    row: &Ref<Grid>,
    vm: Rc<AppViewModel>,
    identity: fn(&Control) -> Option<Id>,
) {
    let model = vm.clone();
    lifetime::attach(&row.clone().upcast(), vm, move |anchor| {
        identity(anchor).map_or_else(Vec::new, |id| task::items(model.clone(), id))
    });
}

pub(crate) fn attach_list(button: &Ref<Button>, vm: Rc<AppViewModel>, id: Id) {
    let model = vm.clone();
    lifetime::attach(&button.clone().upcast(), vm, move |_| {
        list::items(model.clone(), id)
    });
}

/// Detail panes retain their toolbar while the selected task changes.
pub(crate) fn task_options(button: &Ref<Button>, vm: Rc<AppViewModel>) {
    let model = vm.clone();
    lifetime::attach(&button.clone().upcast(), vm, move |_| {
        model
            .selected()
            .map_or_else(Vec::new, |id| task::items(model.clone(), id))
    });
    open_on_click(button);
}

pub(crate) fn header_options(button: &Ref<Button>, vm: Rc<AppViewModel>) {
    let model = vm.clone();
    lifetime::attach(&button.clone().upcast(), vm, move |_| {
        header::items(model.clone())
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
