use super::{feedback, list_id, move_to, payload, reorder, Identity};
use ferroui_base::input::{DragDrop, DragDropEffects, DragEventArgs};
use ferroui_base::Ref;
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{Button, Control, Grid};
use ferroui_view::{on_cleanup, style};
use std::rc::Rc;
use todo_domain::{Action, Id};
use todo_state::AppViewModel;

#[derive(Clone, Copy)]
enum Target {
    Row(Identity),
    List,
}

pub(crate) fn attach_task_target(control: &Ref<Grid>, vm: Rc<AppViewModel>, identity: Identity) {
    attach(&control.clone().upcast(), vm, Target::Row(identity));
}

pub(crate) fn attach_list_target(control: &Ref<Button>, vm: Rc<AppViewModel>) {
    control.styles().add(
        style("Button.todo-drop-target")
            .set(TemplatedControl::background_property(), "#263D73DD")
            .set(TemplatedControl::border_brush_property(), "#3D73DD")
            .build(),
    );
    attach(&control.clone().upcast(), vm, Target::List);
}

fn plan(
    control: &Control,
    vm: &AppViewModel,
    target: Target,
    id: Id,
    event: &DragEventArgs,
) -> Option<(Action, bool)> {
    let point = event.get_position(control);
    let bounds = control.bounds();
    if point.x < 0.0 || point.x > bounds.width || point.y < 0.0 || point.y > bounds.height {
        return None;
    }
    let after = point.y >= bounds.height / 2.0;
    let action = match target {
        Target::Row(identity) => reorder(vm, id, identity(control)?, after),
        Target::List => move_to(vm, id, list_id(control)?),
    }?;
    Some((action, after))
}

fn attach(control: &Ref<Control>, vm: Rc<AppViewModel>, target: Target) {
    DragDrop::set_allow_drop(control, true);
    let weak = control.downgrade();
    let hover_vm = vm.clone();
    let hover = move |event: &DragEventArgs, scrolling: bool| {
        let Some(control) = weak.upgrade() else {
            return;
        };
        let planned = payload(event, &hover_vm)
            .and_then(|drag| plan(&control, &hover_vm, target, drag.task_id, event));
        event.set_drag_effects(if planned.is_some() {
            DragDropEffects::MOVE
        } else {
            DragDropEffects::NONE
        });
        feedback::show(
            &control,
            planned.is_some(),
            planned.is_some_and(|(_, after)| after),
        );
        if scrolling && event.drag_effects() == DragDropEffects::MOVE {
            feedback::scroll(&control, event);
        }
        event.set_handled(true);
    };
    let enter_hover = hover.clone();
    let enter =
        DragDrop::add_drag_enter_handler(control, move |_, event| enter_hover(event, false));
    let over = DragDrop::add_drag_over_handler(control, move |_, event| hover(event, true));
    let weak = control.downgrade();
    let leave = DragDrop::add_drag_leave_handler(control, move |_, _| {
        if let Some(control) = weak.upgrade() {
            feedback::show(&control, false, false);
        }
    });
    let weak = control.downgrade();
    let drop = DragDrop::add_drop_handler(control, move |_, event| {
        let Some(control) = weak.upgrade() else {
            return;
        };
        feedback::show(&control, false, false);
        event.set_drag_effects(DragDropEffects::NONE);
        event.set_handled(true);
        let Some(drag) = payload(event, &vm) else {
            return;
        };
        let Some((action, _)) = plan(&control, &vm, target, drag.task_id, event) else {
            return;
        };
        if drag.active.replace(false) {
            vm.dispatch(action);
            event.set_drag_effects(DragDropEffects::MOVE);
        }
    });
    let weak = control.downgrade();
    on_cleanup(move || {
        if let Some(control) = weak.upgrade() {
            feedback::show(&control, false, false);
            DragDrop::remove_drag_enter_handler(&control, enter);
            DragDrop::remove_drag_over_handler(&control, over);
            DragDrop::remove_drag_leave_handler(&control, leave);
            DragDrop::remove_drop_handler(&control, drop);
        }
    });
}
