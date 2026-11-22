mod focus;
mod focus_target;
mod input;
mod key_route;
mod keyboard;
mod title_bar;

use crate::{detail, dialogs, list, palette, rows::Rows, settings, sidebar, suggestions};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::TextWrapping;
use ferroui_controls::Grid;
use ferroui_view::prelude::*;
pub(crate) use input::Drafts;
use std::rc::Rc;
use todo_controls::{icon_button, Icon};
use todo_state::{AppViewModel, Panel, NAVIGATION_WIDTH};

pub fn view(vm: Rc<AppViewModel>, rows: Rc<Rows>) -> Element {
    let drafts = use_hook(|| Rc::new(Drafts::new(vm.view())));
    drafts.synchronize_route(vm.view());
    let root_node = use_hook(NodeRef::<Grid>::new);
    let focus = use_hook(|| Rc::new(focus::PaneFocus::default()));
    let layout = vm.layout();
    focus.synchronize(&vm, &rows, &drafts, root_node, layout);
    let p = palette(&vm);
    let panel = vm.panel();
    let side_panel = matches!(panel, Panel::Suggestions | Panel::Settings);
    let left_width = if layout.navigation_inline { "280" } else { "0" };
    let center_width = if layout.list_visible { "*" } else { "0" };
    let right_width = if layout.right_visible {
        if layout.compact {
            "*"
        } else {
            "340"
        }
    } else {
        "0"
    };
    let keys = vm.clone();
    let key_drafts = drafts.clone();
    let mut root = grid()
        .name("TodoApp")
        .node_ref(root_node)
        .focusable(true)
        .is_tab_stop(false)
        .background(p.background)
        .row_definitions("36,*")
        .with(move |root| key_route::attach(root, keys.clone(), key_drafts.clone()))
        .child(title_bar::view(vm.clone()));
    let mut body = grid()
        .grid_row(1)
        .column_definitions(&format!("{left_width},{center_width},{right_width}"));
    if layout.navigation_inline {
        body = body.child(
            grid()
                .key("sidebar")
                .child(sidebar::view(vm.clone(), drafts.clone())),
        );
    }
    if layout.list_visible {
        body = body.child(grid().key("task-list").grid_column(1).child(list::view(
            vm.clone(),
            rows,
            drafts.clone(),
        )));
    }
    if side_panel {
        body = body.child(grid().key("side-panel").grid_column(2).child(
            if panel == Panel::Settings {
                settings::view(vm.clone())
            } else {
                suggestions::view(vm.clone())
            },
        ));
    } else if let Some(id) = vm.selected() {
        body = body.child(
            grid()
                .key("task-detail")
                .grid_column(2)
                .child(detail::view(vm.clone(), id)),
        );
    }
    root = root.child(body);
    if !vm.status().is_empty() {
        let close = vm.clone();
        let undo = vm.clone();
        root = root.child(
            border()
                .grid_row(1)
                .vertical_alignment(V::Bottom)
                .horizontal_alignment(H::Stretch)
                .margin((16.0, 16.0))
                .padding((14.0, 8.0))
                .corner_radius(5.0)
                .background(p.surface)
                .border_brush(p.border)
                .border_thickness(1.0)
                .max_width(650.0)
                .child(
                    grid()
                        .column_definitions("*,Auto,Auto")
                        .column_spacing(12.0)
                        .child(
                            text_block()
                                .text(vm.status())
                                .font_size(13.0)
                                .foreground(p.text)
                                .text_wrapping(TextWrapping::Wrap),
                        )
                        .child(
                            border()
                                .grid_column(1)
                                .child(todo_controls::text_button("Undo", move || undo.undo())),
                        )
                        .child(border().grid_column(2).child(icon_button(
                            Icon::Close,
                            "Dismiss message",
                            move || close.clear_status(),
                        ))),
                ),
        );
    }
    if layout.navigation_overlay {
        let dismiss = vm.clone();
        let close = vm.clone();
        root = root.child(
            grid()
                .key("navigation-drawer")
                .grid_row(1)
                .child(
                    button()
                        .class("todo-flat")
                        .background("#55000000")
                        .focusable(false)
                        .is_tab_stop(false)
                        .horizontal_alignment(H::Stretch)
                        .vertical_alignment(V::Stretch)
                        .automation_properties_name("Close navigation")
                        .on_click(move |_| dismiss.close_navigation()),
                )
                .child(
                    border()
                        .name("NavigationPane")
                        .width(NAVIGATION_WIDTH)
                        .horizontal_alignment(H::Left)
                        .background(p.sidebar)
                        .keyboard_navigation_tab_navigation(
                            ferroui_base::input::KeyboardNavigationMode::Cycle,
                        )
                        .child(
                            grid()
                                .row_definitions("40,*")
                                .child(
                                    border()
                                        .horizontal_alignment(H::Left)
                                        .margin((8.0, 4.0))
                                        .child(icon_button(
                                            Icon::Menu,
                                            "Hide navigation",
                                            move || close.close_navigation(),
                                        )),
                                )
                                .child(grid().grid_row(1).child(sidebar::view(vm.clone(), drafts))),
                        ),
                ),
        );
    }
    if !matches!(panel, Panel::None | Panel::Settings | Panel::Suggestions) {
        root = root.child(dialogs::view(vm).grid_overlay());
    }
    root.into()
}

trait Overlay {
    fn grid_overlay(self) -> Element;
}
impl Overlay for Element {
    fn grid_overlay(self) -> Element {
        grid().grid_row(1).child(self).into()
    }
}
