use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{icon, Icon};
use todo_state::AppViewModel;

pub(super) fn view(
    vm: Rc<AppViewModel>,
    label: String,
    completed: bool,
    expanded: bool,
    color: &str,
) -> Element {
    let caption = text_block()
        .text(label.clone())
        .font_size(13.0)
        .font_weight(FontWeight::SemiBold)
        .foreground(color);
    if !completed {
        return caption
            .margin((8.0, 14.0, 8.0, 8.0))
            .automation_properties_heading_level(2)
            .into();
    }
    button()
        .class(if color == "#FFFFFF" {
            "todo-flat todo-on-color"
        } else {
            "todo-flat"
        })
        .height(38.0)
        .padding((8.0, 4.0))
        .margin((0.0, 10.0, 0.0, 2.0))
        .horizontal_content_alignment(H::Left)
        .automation_properties_name(label)
        .automation_properties_item_status(if expanded { "Expanded" } else { "Collapsed" })
        .on_click(move |_| vm.toggle_completed())
        .child(
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(10.0)
                .child(icon(
                    if expanded {
                        Icon::ChevronDown
                    } else {
                        Icon::ChevronRight
                    },
                    12.0,
                    color,
                ))
                .child(caption),
        )
        .into()
}
