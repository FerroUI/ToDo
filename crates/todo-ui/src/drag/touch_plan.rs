use super::{list_id, move_to, reorder, sidebar, sidebar_feedback::Placement, DragItem};
use ferroui_base::{Point, Ref};
use ferroui_controls::{Control, TopLevel};
use todo_domain::Action;
use todo_state::{gestures::drag::DragOrigin, AppViewModel};

#[derive(Clone, Copy)]
pub(super) enum Cue {
    Task(bool),
    Sidebar(Placement),
}

pub(super) struct Plan {
    pub control: Ref<Control>,
    pub action: Action,
    pub cue: Cue,
}

pub(super) fn plan(
    root: &TopLevel,
    vm: &AppViewModel,
    origin: &DragOrigin,
    position: Point,
) -> Option<Plan> {
    if !origin.valid(vm) {
        return None;
    }
    let hit = root.input_hit_test_with(position, false)?;
    for control in hit
        .get_self_and_visual_ancestors()
        .filter_map(|visual| visual.cast::<Control>())
    {
        let point = root.translate_point(position, &control)?;
        let after = point.y >= control.bounds().height / 2.0;
        let planned = match origin.item {
            DragItem::Task(id) => {
                if let Some(target) = crate::touch::task_id(&control) {
                    reorder(vm, id, target, after).map(|action| (action, Cue::Task(after)))
                } else if let Some(list) = list_id(&control) {
                    move_to(vm, id, list).map(|action| (action, Cue::Task(false)))
                } else {
                    None
                }
            }
            DragItem::Sidebar(item) => sidebar::target(&control)
                .and_then(|target| sidebar::plan(&control, vm, item, target, point))
                .map(|(action, placement)| (action, Cue::Sidebar(placement))),
        };
        if let Some((action, cue)) = planned {
            return Some(Plan {
                control,
                action,
                cue,
            });
        }
    }
    None
}
