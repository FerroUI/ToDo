use crate::palette;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{icon, toolbar_button, Icon};
use todo_domain::{Action, ListGroup};
use todo_state::{AppViewModel, RenameTarget};

pub(super) fn header(vm: Rc<AppViewModel>, group: &ListGroup) -> Element {
    let p = palette(&vm);
    let id = group.id;
    let target = RenameTarget::Group(id);
    let title: Element = if vm
        .rename_session()
        .is_some_and(|edit| edit.target == target)
    {
        crate::inline_name::editor(vm.clone(), target, 14.0, p.text)
    } else {
        let toggle = vm.clone();
        let context = vm.clone();
        button()
            .key(format!("group-title-{id}"))
            .name(format!("Group{id}"))
            .class("todo-flat")
            .height(40.0)
            .horizontal_content_alignment(H::Stretch)
            .automation_properties_name(&group.name)
            .with(move |button| crate::menus::attach_group(button, context.clone(), id))
            .on_click(move |_| toggle.dispatch(Action::ToggleGroup(id)))
            .child(
                grid()
                    .column_definitions("26,*")
                    .child(border().vertical_alignment(V::Center).child(icon(
                        if group.collapsed {
                            Icon::ChevronRight
                        } else {
                            Icon::ChevronDown
                        },
                        14.0,
                        p.secondary,
                    )))
                    .child(
                        text_block()
                            .grid_column(1)
                            .vertical_alignment(V::Center)
                            .text(&group.name)
                            .font_size(14.0)
                            .foreground(p.text)
                            .text_trimming(
                                <dyn ferroui_base::media::TextTrimming>::character_ellipsis(),
                            ),
                    ),
            )
            .into()
    };
    grid()
        .key(format!("group-{id}"))
        .column_definitions("*,Auto")
        .margin((0.0, 6.0, 0.0, 0.0))
        .child(title)
        .child(
            toolbar_button(Icon::More, "Group options", p.secondary)
                .name(format!("GroupOptions{id}"))
                .grid_column(1)
                .vertical_alignment(V::Center)
                .with(move |button| crate::menus::group_options(button, vm.clone(), id)),
        )
        .into()
}
