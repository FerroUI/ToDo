use crate::palette;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{Icon, icon, toolbar_button};
use todo_domain::{Action, Id, ListGroup, SidebarItem};
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
        let drag = vm.clone();
        button()
            .key(format!("group-title-{id}"))
            .name(format!("Group{id}"))
            .class("todo-flat")
            .height(40.0)
            .horizontal_content_alignment(H::Stretch)
            .vertical_content_alignment(V::Stretch)
            .automation_properties_name(&group.name)
            .tag(Some(
                Rc::new(SidebarItem::Group(id)) as ferroui_base::BoxedValue
            ))
            .automation_properties_item_status(if group.collapsed {
                "Collapsed"
            } else {
                "Expanded"
            })
            .automation_properties_help_text(
                "Use Left and Right to collapse or expand. Press Shift+F10 for group options.",
            )
            .tool_tip_tip(if group.collapsed {
                "Expand group"
            } else {
                "Collapse group"
            })
            .with(move |button| {
                crate::menus::attach_group(button, context.clone(), id);
                crate::drag::attach_sidebar_source(button, drag.clone());
                crate::drag::attach_sidebar_target(
                    button,
                    drag.clone(),
                    crate::drag::SidebarTarget::Row,
                );
            })
            .on_click(move |event| {
                if crate::drag::ignore_click(event) {
                    event.set_handled(true);
                    return;
                }
                toggle.dispatch(Action::ToggleGroup(id));
            })
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
                    )
                    .child(crate::drag::sidebar_marker(p.accent, false))
                    .child(crate::drag::sidebar_marker(p.accent, true)),
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

pub(super) fn empty(vm: Rc<AppViewModel>, id: Id) -> Element {
    let p = palette(&vm);
    button()
        .name(format!("EmptyGroup{id}"))
        .class("todo-flat")
        .min_height(38.0)
        .margin((22.0, 0.0, 8.0, 6.0))
        .padding((8.0, 6.0))
        .focusable(false)
        .is_tab_stop(false)
        .horizontal_content_alignment(H::Left)
        .automation_properties_name("Drag lists here")
        .tool_tip_tip("Drag a list here, or use Group options to add a list")
        .with(move |button| {
            crate::drag::attach_sidebar_target(
                button,
                vm.clone(),
                crate::drag::SidebarTarget::GroupContents(id),
            )
        })
        .child(
            text_block()
                .text("Drag lists here")
                .font_size(12.0)
                .foreground(p.secondary),
        )
        .into()
}
