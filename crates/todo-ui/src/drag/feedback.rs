use ferroui_base::input::DragEventArgs;
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::Vector;
use ferroui_controls::{Border, Control, ScrollViewer};
use ferroui_view::prelude::*;

pub(super) const TARGET: &str = "todo-drop-target";
const BEFORE: &str = "TaskDropBefore";
const AFTER: &str = "TaskDropAfter";

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

pub(super) fn show(control: &Control, valid: bool, after: bool) {
    control.classes().set(TARGET, valid);
    for (name, visible) in [(BEFORE, valid && !after), (AFTER, valid && after)] {
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
            control.classes().set(TARGET, false);
            if matches!(control.name().as_deref(), Some(BEFORE | AFTER)) {
                control.set_is_visible(false);
            }
        }
    }
}

/// Raw DragDrop consumes pointer moves, so scroll the containing viewport from DragOver itself.
pub(super) fn scroll(control: &Control, event: &DragEventArgs) {
    let Some(scroll) = control.find_ancestor_of_type::<ScrollViewer>(false) else {
        return;
    };
    let point = event.get_position(&scroll);
    let height = scroll.bounds().height;
    let delta = if point.y < 28.0 {
        -16.0
    } else if point.y > height - 28.0 {
        16.0
    } else {
        return;
    };
    let offset = scroll.offset();
    let maximum = (scroll.extent().height - scroll.viewport().height).max(0.0);
    let y = (offset.y + delta).clamp(0.0, maximum);
    if y != offset.y {
        scroll.set_offset(Vector::new(offset.x, y));
    }
}
