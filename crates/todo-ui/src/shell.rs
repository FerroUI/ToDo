mod input;
mod keyboard;
mod title_bar;

use crate::{detail, dialogs, list, palette, rows::Rows, settings, sidebar, suggestions};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_view::prelude::*;
pub(crate) use input::Drafts;
use std::rc::Rc;
use todo_controls::{icon_button, Icon};
use todo_state::{AppViewModel, Panel};

pub fn view(vm: Rc<AppViewModel>, rows: Rc<Rows>) -> Element {
    let drafts = use_hook(|| Rc::new(Drafts::new(vm.view())));
    drafts.synchronize_route(vm.view());
    let p = palette(&vm);
    let narrow = vm.is_narrow();
    let sidebar_requested = vm.sidebar_visible();
    let detail_visible = vm.selected().is_some();
    let panel = vm.panel();
    let side_panel = matches!(panel, Panel::Suggestions | Panel::Settings);
    let sidebar_visible = sidebar_requested && !(narrow && (detail_visible || side_panel));
    let left_width = if sidebar_visible {
        if narrow {
            "*"
        } else {
            "280"
        }
    } else {
        "0"
    };
    let show_list = !(narrow && (sidebar_visible || detail_visible || side_panel));
    let center_width = if show_list { "*" } else { "0" };
    let right_width = if detail_visible || side_panel {
        if narrow {
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
        .focusable(true)
        .is_tab_stop(false)
        .background(p.background)
        .row_definitions("36,*")
        .with(move |root| keyboard::attach(root, keys.clone(), key_drafts.clone()))
        .child(title_bar::view(vm.clone()));
    let mut body = grid()
        .grid_row(1)
        .column_definitions(&format!("{left_width},{center_width},{right_width}"));
    if sidebar_visible {
        body = body.child(
            grid()
                .key("sidebar")
                .child(sidebar::view(vm.clone(), drafts.clone())),
        );
    }
    if show_list {
        body = body.child(grid().key("task-list").grid_column(1).child(list::view(
            vm.clone(),
            rows,
            drafts,
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
                .horizontal_alignment(H::Center)
                .margin((16.0, 16.0))
                .padding((14.0, 8.0))
                .corner_radius(5.0)
                .background(p.surface)
                .border_brush(p.border)
                .border_thickness(1.0)
                .max_width(650.0)
                .child(
                    stack_panel()
                        .orientation(Orientation::Horizontal)
                        .spacing(12.0)
                        .child(
                            text_block()
                                .text(vm.status())
                                .font_size(13.0)
                                .foreground(p.text)
                                .max_width(480.0),
                        )
                        .child(todo_controls::text_button("Undo", move || undo.undo()))
                        .child(icon_button(Icon::Close, "Dismiss message", move || {
                            close.clear_status()
                        })),
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
