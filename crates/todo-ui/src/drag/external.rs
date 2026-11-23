//! Portable imports use the platform Drop lease only for synchronous materialization.
//! They never accept an internal drag's move authority or an external cut instruction.
use super::{feedback, format, list_id};
use ferroui_base::input::{DataTransferExtensions, DragDrop, DragDropEffects, DragEventArgs};
use ferroui_base::Ref;
use ferroui_controls::{Border, Button, Control, Grid, ListBox, RichEditBox, TextBox};
use ferroui_view::on_cleanup;
use std::rc::Rc;
use todo_domain::View;
use todo_state::{AppViewModel, Panel};

mod lifetime;
pub(crate) use lifetime::{cancel_active, has_active};

#[cfg(test)]
mod tests;

pub(crate) const HINT: &str = "ExternalTaskDropHint";

#[derive(Clone, Copy)]
enum Destination {
    CurrentView,
    List,
}

pub(crate) fn attach_body(control: &Ref<Grid>, vm: Rc<AppViewModel>) {
    attach(&control.clone().upcast(), vm, Destination::CurrentView);
}

pub(crate) fn attach_list(control: &Ref<Button>, vm: Rc<AppViewModel>) {
    attach(&control.clone().upcast(), vm, Destination::List);
}

fn destination(control: &Control, vm: &AppViewModel, kind: Destination) -> Option<View> {
    if vm.refreshing() || vm.panel() != Panel::None || vm.rename_session().is_some() {
        return None;
    }
    match kind {
        Destination::CurrentView => Some(vm.view()),
        Destination::List => list_id(control).map(View::List),
    }
}

fn native_editor(event: &DragEventArgs) -> bool {
    event
        .source()
        .and_then(|source| source.cast::<Control>())
        .is_some_and(|source| {
            source.get_self_and_visual_ancestors().any(|visual| {
                visual.cast::<TextBox>().is_some() || visual.cast::<RichEditBox>().is_some()
            })
        })
}

fn offered(event: &DragEventArgs) -> bool {
    if event.handled() || native_editor(event) {
        return false;
    }
    // Native metadata providers use the synchronous transfer panic contract too.
    // Restrict containment to that OS read; no view-model work happens here.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // An invalidated internal drag must not fall back to copying its public formats.
        !event.data_transfer().contains(&format())
            && crate::task_transfer::has_supported(&**event.data_transfer())
    }))
    .unwrap_or(false)
}

fn show(control: &Control, visible: bool) {
    control.classes().set(feedback::TARGET, visible);
    if let Some(hint) =
        control.find_descendant_of_type_where::<Border>(false, |hint| hint.has_name(HINT))
    {
        hint.set_is_visible(visible);
    }
}

fn attach(control: &Ref<Control>, vm: Rc<AppViewModel>, kind: Destination) {
    DragDrop::set_allow_drop(control, true);
    let target = lifetime::Target::new(control, &vm);
    let weak = control.downgrade();
    let hover_vm = vm.clone();
    let hover_target = target.clone();
    let hover = move |event: &DragEventArgs, entering| {
        if entering {
            hover_target.enter();
        }
        if !offered(event) {
            hover_target.cancel();
            return;
        }
        let Some(control) = weak.upgrade() else {
            hover_target.cancel();
            return;
        };
        // A canceled transfer stays canceled until the platform delivers a new
        // DragEnter. Late DragOver messages cannot bring its feedback back.
        let enabled = hover_target.hovering() && destination(&control, &hover_vm, kind).is_some();
        event.set_drag_effects(if enabled {
            DragDropEffects::COPY
        } else {
            DragDropEffects::NONE
        });
        if enabled {
            show(&control, true);
            if entering {
                hover_target.synchronize();
            }
        } else {
            hover_target.cancel();
        }
        event.set_handled(true);
    };
    let entering = hover.clone();
    let enter = DragDrop::add_drag_enter_handler(control, move |_, event| entering(event, true));
    let over = DragDrop::add_drag_over_handler(control, move |_, event| hover(event, false));
    let leaving = target.clone();
    let leave = DragDrop::add_drag_leave_handler(control, move |_, _| {
        leaving.cancel();
    });
    let weak = control.downgrade();
    let dropping = target.clone();
    let dropped = DragDrop::add_drop_handler(control, move |_, event| {
        if event.handled() || native_editor(event) {
            dropping.cancel();
            return;
        }
        event.set_drag_effects(DragDropEffects::NONE);
        let Some(lease) = dropping.begin_drop() else {
            return;
        };
        if !offered(event) || !lease.active() {
            return;
        }
        event.set_handled(true);
        let Some(control) = weak.upgrade() else {
            return;
        };
        show(&control, false);
        let Some(view) = destination(&control, &vm, kind) else {
            return;
        };
        if !lease.active() {
            return;
        }
        let target = vm.task_paste_target(view.clone());
        // A native IDataObject can pump messages while fulfilling GetData. Revalidate
        // both the document epoch and the recycled visual identity after the read.
        let input = crate::task_transfer::read_drop(&**event.data_transfer());
        if !lease.active() {
            return;
        }
        if !control.is_attached_to_visual_tree()
            || destination(&control, &vm, kind).as_ref() != Some(&view)
        {
            vm.report_error("The drop destination changed. Drop the tasks again.".into());
            return;
        }
        let result = input.and_then(|(data, text)| {
            // The drop owns task focus. Blur commits any previous editor before this
            // transaction, so the next Ctrl+Z undoes the imported tasks as one action.
            if let Some(list) = control
                .find_descendant_of_type_where::<ListBox>(false, |list| list.has_name("TaskList"))
            {
                list.focus();
            } else {
                control.focus();
            }
            if !lease.active() {
                return Ok(false);
            }
            if !control.is_attached_to_visual_tree()
                || destination(&control, &vm, kind).as_ref() != Some(&view)
            {
                return Err("The drop destination changed. Drop the tasks again.".into());
            }
            lease.commit(|| vm.paste_external_tasks(target, data, &text))
        });
        match result {
            // A successful transaction is already committed. Notifications may
            // cancel its lease afterward, but the source must still see COPY.
            Ok(true) => event.set_drag_effects(DragDropEffects::COPY),
            Ok(false) => {}
            Err(error) => vm.report_error(error),
        }
    });
    let detached_target = Rc::downgrade(&target);
    let detached = control.detached_from_visual_tree(move |_| {
        if let Some(target) = detached_target.upgrade() {
            target.cancel();
        }
    });
    let weak = control.downgrade();
    on_cleanup(move || {
        target.cancel();
        detached.dispose();
        if let Some(control) = weak.upgrade() {
            DragDrop::remove_drag_enter_handler(&control, enter);
            DragDrop::remove_drag_over_handler(&control, over);
            DragDrop::remove_drag_leave_handler(&control, leave);
            DragDrop::remove_drop_handler(&control, dropped);
        }
    });
}
