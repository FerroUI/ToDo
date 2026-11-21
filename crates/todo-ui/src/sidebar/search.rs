use crate::palette;
use ferroui_base::input::Key;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{icon_button, Icon};
use todo_state::AppViewModel;
pub(super) fn view(
    vm: Rc<AppViewModel>,
    drafts: Rc<crate::shell::Drafts>,
) -> El<ferroui_controls::Grid> {
    let p = palette(&vm);
    let search = drafts.search;
    let update = vm.clone();
    let typing = vm.clone();
    let changed = drafts.clone();
    let enter = drafts.clone();
    grid()
        .column_definitions("*,30")
        .margin((16.0, 0.0, 16.0, 12.0))
        .child(
            text_box()
                .name("SearchTasks")
                .node_ref(drafts.search_input)
                .text(search.two_way())
                .placeholder_text("Search")
                .automation_properties_name("Search tasks, steps, notes and tags")
                .height(32.0)
                .font_size(13.0)
                .padding((8.0, 4.0))
                .background(p.surface)
                .on_changed(ferroui_controls::TextBox::text_property(), move |value| {
                    changed.search_changed(typing.clone(), value.clone().unwrap_or_default());
                })
                .on_key_down(move |event| {
                    if event.key == Key::Return {
                        enter.submit_search(&update);
                        event.set_handled(true);
                    }
                }),
        )
        .child(border().grid_column(1).child(icon_button(
            Icon::Search,
            "Search tasks",
            move || drafts.submit_search(&vm),
        )))
}
