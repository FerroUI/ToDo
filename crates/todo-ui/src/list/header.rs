use ferroui_base::layout::{Orientation, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::View;
use todo_state::{AppViewModel, Panel};
pub(super) fn view(vm: Rc<AppViewModel>, foreground: &str, inset: f64) -> Element {
    let my_day = matches!(vm.view(), View::MyDay);
    let menu = vm.clone();
    let suggest = vm.clone();
    let sort = vm.clone();
    let show_nav = vm.clone();
    let select = vm.clone();
    let mut header = grid()
        .column_definitions("*,Auto")
        .margin((inset, 20.0, inset, 22.0));
    let title = stack_panel()
        .spacing(5.0)
        .child(
            text_block()
                .text(vm.title())
                .font_size(30.0)
                .font_weight(FontWeight::SemiBold)
                .text_trimming(<dyn ferroui_base::media::TextTrimming>::character_ellipsis())
                .foreground(foreground),
        )
        .child(
            text_block()
                .text(vm.subtitle())
                .font_size(13.0)
                .foreground(foreground)
                .opacity(0.92),
        );
    header = header.child(
        stack_panel()
            .spacing(8.0)
            .child(if !vm.sidebar_visible() {
                todo_controls::colored_icon_button(
                    Icon::Menu,
                    "Show navigation",
                    foreground,
                    move || show_nav.toggle_sidebar(),
                )
            } else {
                Element::Empty
            })
            .child(title),
    );
    let mut toolbar = stack_panel()
        .grid_column(1)
        .orientation(Orientation::Horizontal)
        .spacing(4.0)
        .vertical_alignment(V::Top);
    if my_day {
        toolbar = toolbar.child(todo_controls::colored_icon_button(
            Icon::Lightbulb,
            "Suggestions",
            foreground,
            move || suggest.show_panel(Panel::Suggestions),
        ));
    } else {
        toolbar = toolbar.child(todo_controls::colored_icon_button(
            Icon::Sort,
            "Sort tasks",
            foreground,
            move || sort.show_panel(Panel::Sort),
        ));
    }
    toolbar = toolbar.child(todo_controls::colored_icon_button(
        Icon::CheckCircle,
        "Select tasks",
        foreground,
        move || select.begin_selection(),
    ));
    toolbar = toolbar.child(todo_controls::colored_icon_button(
        Icon::More,
        "List options",
        foreground,
        move || menu.show_panel(Panel::ListMenu),
    ));
    header = header.child(toolbar);
    header.into()
}
