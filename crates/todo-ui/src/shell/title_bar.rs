use crate::palette;
use ferroui_base::layout::VerticalAlignment as V;
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{icon, icon_button, Icon};
use todo_state::{AppViewModel, Panel};
pub(super) fn view(vm: Rc<AppViewModel>) -> Element {
    let p = palette(&vm);
    let preferences = vm.clone();
    grid()
        .column_definitions("Auto,*,Auto")
        .background(p.sidebar)
        .height(36.0)
        .child(
            border()
                .padding((16.0, 8.0))
                .child(icon(Icon::Check, 20.0, "#2564CF")),
        )
        .child(
            text_block()
                .grid_column(1)
                .text("To Do")
                .font_size(12.0)
                .font_weight(FontWeight::SemiBold)
                .foreground(p.text)
                .vertical_alignment(V::Center),
        )
        .child(
            border()
                .grid_column(2)
                .margin((0.0, 0.0, 8.0, 0.0))
                .child(icon_button(Icon::Settings, "Settings", move || {
                    preferences.show_panel(Panel::Settings)
                })),
        )
        .into()
}
