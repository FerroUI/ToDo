use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::TextWrapping;
use ferroui_controls::Control;
use ferroui_view::prelude::*;
use todo_controls::{Icon, inherited_icon, toolbar_button};

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

/// Removing a conditional clear control keeps keyboard navigation in its row.
pub(super) fn clear_button(
    label: &str,
    color: &str,
    retained_name: &'static str,
    on_clear: impl Fn() + 'static,
) -> Element {
    toolbar_button(Icon::Close, label, color)
        .on_click(move |event| {
            event.set_handled(true);
            if let Some(action) = event
                .source()
                .and_then(|source| source.cast::<Control>())
                .and_then(|source| source.get_visual_parent_of_type::<Grid>())
                .and_then(|row| {
                    row.find_descendant_of_type_where::<Button>(false, |button| {
                        button.name().as_deref() == Some(retained_name)
                    })
                })
            {
                action.focus();
            }
            on_clear();
        })
        .into()
}
