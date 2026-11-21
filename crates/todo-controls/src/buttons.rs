//! Small declarative components that keep the semantics of native FerroUI Buttons.

use crate::icons::{inherited_icon, Icon};
use crate::Palette;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
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
    let action = icon_action(kind, label, on_click).foreground(color);
    if matches!(color, "#FFFFFF" | "#ffffff" | "#FFF" | "#fff" | "White") {
        action.class("todo-on-color").into()
    } else {
        action.into()
    }
}

fn icon_action(kind: Icon, label: &str, on_click: impl Fn() + 'static) -> El<Button> {
    button()
        .class("todo-toolbar")
        .width(32.0)
        .height(32.0)
        .padding(6.0)
        .automation_properties_name(label)
        .tool_tip_tip(label)
        .on_click(move |e| {
            e.set_handled(true);
            on_click();
        })
        .child(inherited_icon(kind, 18.0))
}

pub fn text_button(label: &str, on_click: impl Fn() + 'static) -> Element {
    button()
        .class("todo-action")
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
        .horizontal_content_alignment(HorizontalAlignment::Left)
        .automation_properties_name(label)
        .on_click(move |e| {
            e.set_handled(true);
            on_click();
        })
        .child(
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(12.0)
                .child(inherited_icon(kind, 18.0))
                .child(
                    text_block()
                        .text(label)
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
        .into()
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
