use crate::palette;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{icon, Icon};
use todo_state::{AppViewModel, Panel};
pub(super) fn view(vm: Rc<AppViewModel>) -> Element {
    let p = palette(&vm);
    let name = vm.state().settings.account_name.clone();
    let account_label = if vm.cloud_connected() {
        vm.cloud_label()
    } else {
        "Local account".into()
    };
    let account = vm;
    button()
        .class("todo-flat")
        .margin((8.0, 8.0))
        .padding((12.0, 8.0))
        .height(64.0)
        .horizontal_content_alignment(H::Stretch)
        .on_click(move |_| account.show_panel(Panel::Settings))
        .child(
            grid()
                .column_definitions("40,*,Auto")
                .child(
                    border()
                        .width(36.0)
                        .height(36.0)
                        .corner_radius(18.0)
                        .background("#E6EBF6")
                        .child(
                            text_block()
                                .text(
                                    name.chars()
                                        .next()
                                        .unwrap_or('M')
                                        .to_uppercase()
                                        .to_string(),
                                )
                                .font_size(17.0)
                                .foreground("#3C5EAA")
                                .horizontal_alignment(H::Center)
                                .vertical_alignment(V::Center),
                        ),
                )
                .child(
                    stack_panel()
                        .grid_column(1)
                        .margin((8.0, 0.0))
                        .spacing(4.0)
                        .child(
                            text_block()
                                .text(name)
                                .font_weight(FontWeight::SemiBold)
                                .font_size(14.0)
                                .foreground(p.text),
                        )
                        .child(
                            text_block()
                                .text(account_label)
                                .font_size(11.0)
                                .foreground(p.secondary),
                        ),
                )
                .child(
                    border()
                        .grid_column(2)
                        .vertical_alignment(V::Center)
                        .child(icon(Icon::ChevronDown, 12.0, p.secondary)),
                ),
        )
        .into()
}
