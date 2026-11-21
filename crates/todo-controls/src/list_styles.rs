//! Strip only the item chrome. ListBox keeps its virtualizing panel and scroll host.

use crate::templates::list_item_theme;
use ferroui_base::layout::{HorizontalAlignment, Layoutable, VerticalAlignment};
use ferroui_base::StyledElement;
use ferroui_controls::primitives::{ScrollBarVisibility, TemplatedControl};
use ferroui_controls::{Application, ContentControl, ItemsControl, ScrollViewer};
use ferroui_view::style;

pub(crate) fn install(app: &Application) {
    app.styles().add(
        style("ListBox.todo-list")
            .set(TemplatedControl::background_property(), "Transparent")
            .set(TemplatedControl::border_thickness_property(), 0.0)
            .set(TemplatedControl::padding_property(), 0.0)
            .set(
                ItemsControl::item_container_theme_property(),
                Some(list_item_theme()),
            )
            .set(
                ScrollViewer::horizontal_scroll_bar_visibility_property(),
                ScrollBarVisibility::Disabled,
            )
            .set(
                ScrollViewer::vertical_scroll_bar_visibility_property(),
                ScrollBarVisibility::Auto,
            )
            .build(),
    );
    app.styles().add(
        style("ListBox.todo-list ListBoxItem")
            .set(StyledElement::theme_property(), Some(list_item_theme()))
            .set(TemplatedControl::background_property(), "Transparent")
            .set(TemplatedControl::border_brush_property(), "Transparent")
            .set(TemplatedControl::border_thickness_property(), 0.0)
            .set(TemplatedControl::padding_property(), 0.0)
            .set(Layoutable::margin_property(), (0.0, 0.0, 0.0, 2.0))
            .set(Layoutable::min_height_property(), 0.0)
            .set(
                ContentControl::horizontal_content_alignment_property(),
                HorizontalAlignment::Stretch,
            )
            .set(
                ContentControl::vertical_content_alignment_property(),
                VerticalAlignment::Stretch,
            )
            .build(),
    );
}
