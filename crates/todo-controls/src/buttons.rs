//! Small declarative components that keep the semantics of native FerroUI Buttons.

use crate::icons::{inherited_icon, Icon};
use crate::Palette;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;

pub fn icon_button(kind: Icon, label: &str, on_click: impl Fn() + 'static) -> Element {
    icon_action(kind, label, on_click).into()
}

pub fn colored_icon_button(
    kind: Icon,
    label: &str,
    color: &str,
    on_click: impl Fn() + 'static,
) -> Element {
    toolbar_button(kind, label, color)
        .on_click(move |event| {
            event.set_handled(true);
            on_click();
        })
        .into()
}

/// Typed toolbar control, allowing feature components to attach native menus and focus behavior.
pub fn toolbar_button(kind: Icon, label: &str, color: &str) -> El<Button> {
    let action = icon_base(kind, label).foreground(color);
    if matches!(color, "#FFFFFF" | "#ffffff" | "#FFF" | "#fff" | "White") {
        action.class("todo-on-color")
    } else {
        action
    }
}

fn icon_action(kind: Icon, label: &str, on_click: impl Fn() + 'static) -> El<Button> {
    icon_base(kind, label).on_click(move |event| {
        event.set_handled(true);
        on_click();
    })
}

fn icon_base(kind: Icon, label: &str) -> El<Button> {
    button()
        .class("todo-toolbar")
        .width(32.0)
        .height(32.0)
        .padding(6.0)
        .automation_properties_name(label)
        .tool_tip_tip(label)
        .child(inherited_icon(kind, 18.0))
}

pub fn text_button(label: &str, on_click: impl Fn() + 'static) -> Element {
    button()
        .class("todo-action")
        .min_height(44.0)
        .automation_properties_name(label)
        .content(label)
        .on_click(move |e| {
            e.set_handled(true);
            on_click();
        })
        .into()
}

pub fn primary_button(label: &str, on_click: impl Fn() + 'static) -> Element {
    button()
        .class("todo-primary")
        .min_height(44.0)
        .automation_properties_name(label)
        .content(label)
        .on_click(move |e| {
            e.set_handled(true);
            on_click();
        })
        .into()
}

pub fn action_button(kind: Icon, label: &str, on_click: impl Fn() + 'static) -> Element {
    button()
        .class("todo-action")
        .min_height(44.0)
        .padding((12.0, 10.0))
        .horizontal_content_alignment(HorizontalAlignment::Stretch)
        .automation_properties_name(label)
        .on_click(move |e| {
            e.set_handled(true);
            on_click();
        })
        .child(
            grid()
                .column_definitions("24,*")
                .column_spacing(8.0)
                .child(
                    border()
                        .vertical_alignment(VerticalAlignment::Center)
                        .child(inherited_icon(kind, 18.0)),
                )
                .child(
                    text_block()
                        .grid_column(1)
                        .text(label)
                        .font_size(14.0)
                        .text_wrapping(TextWrapping::Wrap)
                        .vertical_alignment(VerticalAlignment::Center),
                ),
        )
        .into()
}

pub fn task_check(checked: bool, color: &str, on_click: impl Fn() + 'static) -> Element {
    let label = if checked {
        "Mark as not completed"
    } else {
        "Mark as completed"
    };
    check_box()
        .class("todo-completion")
        .is_checked(Some(checked))
        .width(40.0)
        .height(40.0)
        .padding(9.0)
        .foreground(color)
        .automation_properties_name(label)
        .tool_tip_tip(label)
        .on_click(move |e| {
            e.set_handled(true);
            on_click();
        })
        .child(inherited_icon(
            if checked {
                Icon::CheckCircle
            } else {
                Icon::Circle
            },
            20.0,
        ))
        .into()
}

pub fn star_button(important: bool, color: &str, on_click: impl Fn() + 'static) -> Element {
    star_action(important, color, on_click).into()
}

pub fn star_action(
    important: bool,
    color: &str,
    on_click: impl Fn() + 'static,
) -> El<ToggleButton> {
    let label = if important {
        "Remove importance"
    } else {
        "Mark as important"
    };
    toggle_button()
        .class("todo-star")
        .is_checked(Some(important))
        .width(32.0)
        .height(32.0)
        .padding(6.0)
        .foreground(color)
        .automation_properties_name(label)
        .tool_tip_tip(label)
        .on_click(move |e| {
            e.set_handled(true);
            on_click();
        })
        .child(inherited_icon(
            if important {
                Icon::StarFilled
            } else {
                Icon::Star
            },
            18.0,
        ))
}

pub fn text(value: &str, size: f64, color: &str) -> Element {
    text_block()
        .text(value)
        .font_size(size)
        .foreground(color)
        .into()
}

pub fn task_card(child: impl IntoChild, selected: bool, dark: bool) -> Element {
    let palette = Palette::for_dark(dark);
    border()
        .class("todo-task")
        .corner_radius(4.0)
        .background(if selected {
            palette.selection
        } else {
            palette.surface
        })
        .min_height(54.0)
        .child(child)
        .into()
}
