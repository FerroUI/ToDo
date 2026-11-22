use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::{View, TASKS_LIST};
use todo_state::{AppViewModel, Panel, RenameTarget};
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
        .child(title(vm.clone(), foreground))
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
            .child(todo_controls::colored_icon_button(
                Icon::Menu,
                if vm.sidebar_visible() {
                    "Hide navigation"
                } else {
                    "Show navigation"
                },
                foreground,
                move || show_nav.toggle_sidebar(),
            ))
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
        toolbar = toolbar.child(
            todo_controls::toolbar_button(Icon::Sort, "Sort tasks", foreground)
                .with(move |button| crate::menus::header_sort(button, sort.clone())),
        );
    }
    toolbar = toolbar.child(todo_controls::colored_icon_button(
        Icon::CheckCircle,
        "Select tasks",
        foreground,
        move || select.begin_selection(),
    ));
    toolbar = toolbar.child(
        todo_controls::toolbar_button(Icon::More, "List options", foreground)
            .with(move |button| crate::menus::header_options(button, menu.clone())),
    );
    header = header.child(toolbar);
    header.into()
}

fn title(vm: Rc<AppViewModel>, foreground: &str) -> Element {
    let text = text_block()
        .text(vm.title())
        .font_size(30.0)
        .font_weight(FontWeight::SemiBold)
        .text_trimming(<dyn ferroui_base::media::TextTrimming>::character_ellipsis())
        .foreground(foreground);
    let View::List(id) = vm.view() else {
        return text.into();
    };
    if id == TASKS_LIST {
        return text.into();
    }
    let target = RenameTarget::List(id);
    if vm
        .rename_session()
        .is_some_and(|edit| edit.target == target)
    {
        return crate::inline_name::editor(vm, target, 30.0, foreground);
    }
    button()
        .key("list-title")
        .name("RenameList")
        .class("todo-flat")
        .padding(0.0)
        .horizontal_content_alignment(H::Stretch)
        .automation_properties_name("Rename list")
        .tool_tip_tip("Rename list")
        .on_click(move |_| vm.begin_rename(target))
        .child(text)
        .into()
}
