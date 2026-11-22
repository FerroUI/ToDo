use crate::palette;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{Palette, ALL_LIST_THEMES};
use todo_domain::{Action, View};
use todo_state::{AppViewModel, Panel};

pub(super) fn view(vm: Rc<AppViewModel>) -> Element {
    let p = palette(&vm);
    let selected_accent = vm.accent();
    let mut swatches = wrap_panel()
        .orientation(Orientation::Horizontal)
        .item_width(104.0)
        .item_height(76.0);
    if !ALL_LIST_THEMES
        .iter()
        .any(|theme| selected_accent.eq_ignore_ascii_case(theme.accent))
    {
        let keep = vm.clone();
        swatches = swatches.child(swatch(
            "Current",
            &selected_accent,
            p.surface_hover,
            true,
            p,
            // Keeping a custom/default color must preserve its background too.
            move || keep.show_panel(Panel::None),
        ));
    }
    for theme in ALL_LIST_THEMES {
        let select = vm.clone();
        let color = theme.accent.to_string();
        swatches = swatches.child(swatch(
            theme.name,
            theme.accent,
            if vm.is_dark() {
                theme.dark_background
            } else {
                theme.background
            },
            selected_accent.eq_ignore_ascii_case(theme.accent),
            p,
            move || {
                if let View::List(id) = select.view() {
                    select.dispatch(Action::SetListTheme {
                        id,
                        color: color.clone(),
                        background: None,
                    });
                } else {
                    select.set_smart_color(color.clone());
                }
                select.show_panel(Panel::None);
            },
        ));
    }
    stack_panel()
        .spacing(12.0)
        .child(
            text_block()
                .text("Choose a color for this list")
                .font_size(13.0)
                .foreground(p.secondary)
                .margin(8.0),
        )
        .child(swatches)
        .into()
}

fn swatch(
    name: &str,
    accent: &str,
    background: &str,
    selected: bool,
    p: Palette,
    choose: impl Fn() + 'static,
) -> Element {
    button()
        .class("todo-flat")
        .width(94.0)
        .height(68.0)
        .margin(4.0)
        .padding(0.0)
        .border_thickness(2.0)
        .border_brush(if selected { p.accent } else { "Transparent" })
        .automation_properties_name(format!("{name} list theme"))
        .automation_properties_item_status(selected.then(|| "Selected".to_string()))
        .tool_tip_tip(name)
        .on_click(move |_| choose())
        .child(
            border()
                .background(background)
                .corner_radius(4.0)
                .padding(10.0)
                .child(
                    stack_panel()
                        .spacing(7.0)
                        .child(
                            border()
                                .height(5.0)
                                .width(54.0)
                                .horizontal_alignment(H::Left)
                                .background(accent)
                                .corner_radius(2.0),
                        )
                        .child(text_block().text(name).font_size(12.0).foreground(p.text)),
                ),
        )
        .into()
}
