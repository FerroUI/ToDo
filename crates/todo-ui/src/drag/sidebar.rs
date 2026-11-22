use super::{feedback, payload, sidebar_feedback, sidebar_identity, DragItem};
use ferroui_base::input::{DragDrop, DragDropEffects, DragEventArgs};
use ferroui_base::{Point, Ref};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{Button, Control};
use ferroui_view::{on_cleanup, style};
use sidebar_feedback::Placement;
use std::rc::Rc;
use todo_domain::{Action, Id, SidebarDrop, SidebarItem};
use todo_state::AppViewModel;

pub(crate) use sidebar_feedback::marker as sidebar_marker;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SidebarTarget {
    Row,
    GroupContents(Id),
    End,
}

pub(super) fn plan(
    control: &Control,
    vm: &AppViewModel,
    source: SidebarItem,
    target: SidebarTarget,
    point: Point,
) -> Option<(Action, Placement)> {
    let bounds = control.bounds();
    if point.x < 0.0 || point.x > bounds.width || point.y < 0.0 || point.y > bounds.height {
        return None;
    }
    let (target, placement) = match target {
        SidebarTarget::GroupContents(id) => (SidebarDrop::IntoGroup(id), Placement::Into),
        SidebarTarget::End => (SidebarDrop::RootEnd, Placement::After),
        SidebarTarget::Row => {
            let item = sidebar_identity(control)?;
            if let SidebarItem::Group(id) = item {
                if matches!(source, SidebarItem::List(_))
                    && point.y >= bounds.height * 0.25
                    && point.y <= bounds.height * 0.75
                {
                    return vm
                        .state()
                        .sidebar_drop_action(source, SidebarDrop::IntoGroup(id))
                        .map(|action| (action, Placement::Into));
                }
            }
            if point.y < bounds.height / 2.0 {
                (SidebarDrop::Before(item), Placement::Before)
            } else {
                (SidebarDrop::After(item), Placement::After)
            }
        }
    };
    vm.state()
        .sidebar_drop_action(source, target)
        .map(|action| (action, placement))
}

pub(super) fn target(control: &Control) -> Option<SidebarTarget> {
    control
        .tag()
        .and_then(|tag| tag.downcast_ref::<SidebarTarget>().copied())
        .or_else(|| sidebar_identity(control).map(|_| SidebarTarget::Row))
}

pub(crate) fn attach_sidebar_target(
    control: &Ref<Button>,
    vm: Rc<AppViewModel>,
    target: SidebarTarget,
) {
    // Row tags keep their typed SidebarItem identity. The two non-row targets have no
    // item identity, so their tag records the shared mouse/touch drop destination.
    if target != SidebarTarget::Row {
        control.set_tag(Some(Rc::new(target)));
    }
    control.styles().add(
        style("Button.todo-sidebar-into")
            .set(TemplatedControl::background_property(), "#263D73DD")
            .build(),
    );
    DragDrop::set_allow_drop(control, true);
    let weak = control.downgrade();
    let hover_vm = vm.clone();
    let hover = move |event: &DragEventArgs| {
        let Some(control) = weak.upgrade() else {
            return;
        };
        let Some(drag) = payload(event, &hover_vm) else {
            event.set_drag_effects(DragDropEffects::NONE);
            return;
        };
        let DragItem::Sidebar(source) = drag.item else {
            event.set_drag_effects(DragDropEffects::NONE);
            return;
        };
        let planned = plan(
            &control,
            &hover_vm,
            source,
            target,
            event.get_position(&control),
        );
        event.set_drag_effects(if planned.is_some() {
            DragDropEffects::MOVE
        } else {
            DragDropEffects::NONE
        });
        sidebar_feedback::show(&control, planned.map(|(_, placement)| placement));
        feedback::scroll(&control, event);
        event.set_handled(true);
    };
    let entered = hover.clone();
    let enter = DragDrop::add_drag_enter_handler(control, move |_, event| entered(event));
    let over = DragDrop::add_drag_over_handler(control, move |_, event| hover(event));
    let weak = control.downgrade();
    let leave = DragDrop::add_drag_leave_handler(control, move |_, _| {
        if let Some(control) = weak.upgrade() {
            sidebar_feedback::show(&control, None);
        }
    });
    let weak = control.downgrade();
    let drop = DragDrop::add_drop_handler(control, move |_, event| {
        let Some(control) = weak.upgrade() else {
            return;
        };
        let Some(drag) = payload(event, &vm) else {
            return;
        };
        let DragItem::Sidebar(source) = drag.item else {
            return;
        };
        sidebar_feedback::show(&control, None);
        event.set_drag_effects(DragDropEffects::NONE);
        event.set_handled(true);
        let Some((action, _)) = plan(&control, &vm, source, target, event.get_position(&control))
        else {
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
            sidebar_feedback::show(&control, None);
            DragDrop::remove_drag_enter_handler(&control, enter);
            DragDrop::remove_drag_over_handler(&control, over);
            DragDrop::remove_drag_leave_handler(&control, leave);
            DragDrop::remove_drop_handler(&control, drop);
        }
    });
}
