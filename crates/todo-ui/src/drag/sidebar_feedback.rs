use ferroui_base::layout::VerticalAlignment;
use ferroui_controls::{Border, Control};
use ferroui_view::prelude::*;

const BEFORE: &str = "SidebarDropBefore";
const AFTER: &str = "SidebarDropAfter";
const INTO: &str = "todo-sidebar-into";

#[derive(Clone, Copy)]
pub(super) enum Placement {
    Before,
    After,
    Into,
}

pub(crate) fn marker(color: &str, after: bool) -> Element {
    border()
        .name(if after { AFTER } else { BEFORE })
        .grid_column_span(4)
        .height(2.0)
        .vertical_alignment(if after {
            VerticalAlignment::Bottom
        } else {
            VerticalAlignment::Top
        })
        .background(color)
        .is_hit_test_visible(false)
        .is_visible(false)
        .into()
}

pub(super) fn show(control: &Control, placement: Option<Placement>) {
    control
        .classes()
        .set(INTO, matches!(placement, Some(Placement::Into)));
    for (name, visible) in [
        (BEFORE, matches!(placement, Some(Placement::Before))),
        (AFTER, matches!(placement, Some(Placement::After))),
    ] {
        if let Some(marker) = control.find_descendant_of_type_where::<Border>(false, |child| {
            child.name().as_deref() == Some(name)
        }) {
            marker.set_is_visible(visible);
        }
    }
}

pub(super) fn clear_root(root: &Control) {
    for visual in root.get_visual_descendants() {
        if let Some(control) = visual.cast::<Control>() {
            control.classes().set(INTO, false);
            if matches!(control.name().as_deref(), Some(BEFORE | AFTER)) {
                control.set_is_visible(false);
            }
        }
    }
}
