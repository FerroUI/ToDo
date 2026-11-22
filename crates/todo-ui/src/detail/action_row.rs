use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;
use todo_controls::{Icon, inherited_icon};

pub(super) fn action(symbol: Icon, label: &str, color: &str) -> El<Button> {
    button()
        .class("todo-action")
        .min_height(48.0)
        .padding((12.0, 10.0))
        .foreground(color)
        .horizontal_content_alignment(H::Stretch)
        .automation_properties_name(label)
        .child(
            grid()
                .column_definitions("24,*")
                .column_spacing(8.0)
                .child(inherited_icon(symbol, 18.0))
                .child(
                    text_block()
                        .grid_column(1)
                        .text(label)
                        .font_size(14.0)
                        .text_wrapping(TextWrapping::Wrap)
                        .vertical_alignment(V::Center),
                ),
        )
}

pub(super) fn row(action: impl IntoChild, clear: Option<Element>) -> Element {
    grid()
        .column_definitions(if clear.is_some() { "*,40" } else { "*,0" })
        .child(action)
        .child(
            border()
                .grid_column(1)
                .horizontal_alignment(H::Center)
                .vertical_alignment(V::Center)
                .child(clear.unwrap_or(Element::Empty)),
        )
        .into()
}
