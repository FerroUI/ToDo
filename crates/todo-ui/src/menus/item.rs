use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::StyledElement;
use ferroui_controls::{ContextMenu, MenuItem};
use ferroui_view::prelude::*;
use todo_controls::{inherited_icon, Icon};

pub(crate) fn color_command(
    label: &str,
    color: &str,
    checked: bool,
    action: impl Fn() + 'static,
) -> Element {
    menu_item()
        .header(
            text_block()
                .text(label)
                .max_width(220.0)
                .text_trimming(<dyn ferroui_base::media::TextTrimming>::character_ellipsis()),
        )
        .tool_tip_tip(label)
        .automation_properties_name(format!(
            "{} category {label}",
            if checked { "Remove" } else { "Add" }
        ))
        .toggle_type(ferroui_controls::MenuItemToggleType::CheckBox)
        .is_checked(checked)
        .icon(
            border()
                .width(12.0)
                .height(12.0)
                .corner_radius(3.0)
                .background(color),
        )
        .on_key_down(super::keys::isolate)
        .on_click(move |event| {
            close(event);
            action();
            event.set_handled(true);
        })
        .into()
}

pub(crate) fn command(
    label: impl Into<String>,
    symbol: Icon,
    action: impl Fn() + 'static,
) -> Element {
    base(label, symbol)
        .on_click(move |event| {
            close(event);
            action();
            event.set_handled(true);
        })
        .into()
}

pub(crate) fn destructive(
    label: impl Into<String>,
    symbol: Icon,
    dark: bool,
    action: impl Fn() + 'static,
) -> Element {
    base(label, symbol)
        .foreground(if dark { "#F1707B" } else { "#D13438" })
        .on_click(move |event| {
            close(event);
            action();
            event.set_handled(true);
        })
        .into()
}

pub(crate) fn command_detail(
    label: &str,
    description: &str,
    symbol: Icon,
    action: impl Fn() + 'static,
) -> Element {
    base(label, symbol)
        .automation_properties_name(format!("{label}, {description}"))
        .header(
            grid()
                .column_definitions("*,Auto")
                .column_spacing(20.0)
                .child(text_block().text(label))
                .child(text_block().grid_column(1).text(description).opacity(0.65)),
        )
        .on_click(move |event| {
            close(event);
            action();
            event.set_handled(true);
        })
        .into()
}

pub(super) fn submenu(label: impl Into<String>, symbol: Icon, children: Vec<Element>) -> Element {
    let mut item = base(label, symbol).is_enabled(!children.is_empty());
    for child in children {
        item = item.child(child);
    }
    item.into()
}

fn base(label: impl Into<String>, symbol: Icon) -> El<MenuItem> {
    let label = label.into();
    menu_item()
        .header(label.clone())
        .automation_properties_name(label)
        // WinUI's MenuItem uses an unscaled PathIcon inside its own 16px Viewbox.
        // Automatic dimensions preserve the full geometry before that scale is applied.
        .icon(inherited_icon(symbol, f64::NAN))
        .on_key_down(super::keys::isolate)
}

pub(super) fn close(event: &RoutedEventArgs) {
    let mut node = event
        .source()
        .and_then(|source| source.cast::<StyledElement>());
    while let Some(element) = node {
        if let Some(menu) = element.cast::<ContextMenu>() {
            menu.close();
            break;
        }
        node = element.parent();
    }
}
