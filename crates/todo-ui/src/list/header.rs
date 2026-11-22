use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::{TASKS_LIST, View};
use todo_state::{AppViewModel, Panel, RenameTarget};
pub(super) fn view(
    vm: Rc<AppViewModel>,
    foreground: &str,
    inset: f64,
    drafts: Rc<crate::shell::Drafts>,
) -> Element {
    let foreground = foreground.to_owned();
    rerender(move || content(vm.clone(), &foreground, inset, drafts.clone()))
}

fn content(
    vm: Rc<AppViewModel>,
    foreground: &str,
    inset: f64,
    drafts: Rc<crate::shell::Drafts>,
) -> Element {
    let my_day = matches!(vm.view(), View::MyDay);
    let menu = vm.clone();
    let suggest = vm.clone();
    let sort = vm.clone();
    let show_nav = vm.clone();
    let sort_trigger = use_hook(NodeRef::<ferroui_controls::Button>::new);
    let options_trigger = use_hook(NodeRef::<ferroui_controls::Button>::new);
    let mut header = grid()
        .column_definitions("*,Auto")
        .row_definitions("Auto,Auto,Auto")
        .margin((inset, 20.0, inset, 22.0));
    let title = stack_panel()
        .grid_row(1)
        .grid_column_span(2)
        .margin((0.0, 8.0, 0.0, 0.0))
        .spacing(5.0)
        .child(title(vm.clone(), foreground))
        .child(
            text_block()
                .text(vm.subtitle())
                .font_size(13.0)
                .text_trimming(<dyn ferroui_base::media::TextTrimming>::character_ellipsis())
                .foreground(foreground)
                .opacity(0.92),
        );
    header =
        header
            .child(border().horizontal_alignment(H::Left).child(
                todo_controls::colored_icon_button(
                    Icon::Menu,
                    if vm.sidebar_visible() {
                        "Hide navigation"
                    } else {
                        "Show navigation"
                    },
                    foreground,
                    move || show_nav.toggle_sidebar(),
                ),
            ))
            .child(title);
    let mut toolbar = stack_panel()
        .grid_column(1)
        .orientation(Orientation::Horizontal)
        .spacing(4.0)
        .vertical_alignment(V::Top);
    if my_day {
        toolbar = toolbar.child(
            todo_controls::toolbar_button(Icon::Lightbulb, "Suggestions", foreground)
                .key("header-suggestions")
                .on_click(move |event| {
                    event.set_handled(true);
                    suggest.show_panel(Panel::Suggestions);
                }),
        );
    } else {
        toolbar = toolbar.child(
            todo_controls::toolbar_button(Icon::Sort, "Sort tasks", foreground)
                .key("header-sort")
                .node_ref(sort_trigger)
                .with(move |button| crate::menus::header_sort(button, sort.clone())),
        );
    }
    if matches!(vm.view(), View::Search(_)) {
        let cancel = vm.clone();
        toolbar = toolbar.child(
            todo_controls::toolbar_button(Icon::Close, "Close search", foreground)
                .key("header-close-search")
                .on_click(move |event| {
                    event.set_handled(true);
                    drafts.cancel_search(&cancel);
                }),
        );
    }
    let options = todo_controls::toolbar_button(Icon::More, "List options", foreground)
        .key("header-options")
        .node_ref(options_trigger)
        .with(move |button| crate::menus::header_options(button, menu.clone()));
    toolbar = toolbar.child(options);
    header = header
        .child(toolbar)
        .child(
            border()
                .grid_row(2)
                .grid_column_span(2)
                .child(super::sort_status::view(
                    vm,
                    foreground,
                    if my_day {
                        options_trigger
                    } else {
                        sort_trigger
                    },
                )),
        );
    header.into()
}

fn title(vm: Rc<AppViewModel>, foreground: &str) -> Element {
    let text = text_block()
        .text(vm.title())
        .automation_properties_heading_level(1)
        .font_size(30.0)
        .font_weight(FontWeight::SemiBold)
        .text_trimming(<dyn ferroui_base::media::TextTrimming>::character_ellipsis())
        .max_lines(1)
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
