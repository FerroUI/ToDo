use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{Icon, icon};
use todo_domain::{SortOrder, View};
use todo_state::AppViewModel;

/// The reference app keeps the active sort visible and offers reverse and clear in place.
pub(super) fn view(vm: Rc<AppViewModel>, foreground: &str) -> Element {
    let order = vm.sort();
    if order == SortOrder::Manual {
        return Element::Empty;
    }
    let default_ascending = matches!(order, SortOrder::Alphabetical | SortOrder::DueDate);
    let ascending = default_ascending != vm.sort_reversed();
    let reverse = vm.clone();
    let clear = vm.clone();
    let label = match order {
        SortOrder::Importance => "Sorted by importance",
        SortOrder::DueDate => "Sorted by due date",
        SortOrder::MyDay => "Sorted by added to My Day",
        SortOrder::Alphabetical => "Sorted alphabetically",
        SortOrder::Created => "Sorted by creation date",
        SortOrder::Manual => unreachable!(),
    };
    let mut sort = button()
        .name("ReverseSort")
        .class("todo-flat")
        .horizontal_alignment(H::Left)
        .horizontal_content_alignment(H::Stretch)
        .min_height(30.0)
        .padding((8.0, 4.0))
        .foreground(foreground)
        .automation_properties_name(format!(
            "{label}, {}. Reverse sort order",
            if ascending { "ascending" } else { "descending" }
        ))
        .tool_tip_tip("Reverse sort order")
        .on_click(move |_| reverse.reverse_sort())
        .child(
            grid()
                .column_definitions("*,20")
                .child(
                    text_block()
                        .text(label)
                        .font_size(12.0)
                        .foreground(foreground)
                        .vertical_alignment(V::Center)
                        .text_trimming(
                            <dyn ferroui_base::media::TextTrimming>::character_ellipsis(),
                        ),
                )
                .child(border().grid_column(1).child(icon(
                    if ascending {
                        Icon::ArrowUp
                    } else {
                        Icon::ArrowDown
                    },
                    16.0,
                    foreground,
                ))),
        );
    if vm.view() == View::MyDay {
        sort = sort.class("todo-on-color").background("#33000000");
    } else {
        sort = sort.background(if vm.is_dark() {
            "#18FFFFFF"
        } else {
            "#0D000000"
        });
    }
    grid()
        .column_definitions("*,32")
        .margin((0.0, 12.0, 0.0, 0.0))
        .child(sort)
        .child(
            border()
                .grid_column(1)
                .child(todo_controls::colored_icon_button(
                    Icon::Close,
                    "Remove sort",
                    foreground,
                    move || clear.set_sort(SortOrder::Manual),
                )),
        )
        .into()
}
