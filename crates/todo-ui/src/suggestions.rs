use crate::palette;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{icon, icon_button, Icon};
use todo_domain::Action;
use todo_state::{AppViewModel, Panel};

pub fn view(vm: Rc<AppViewModel>) -> Element {
    let p = palette(&vm);
    let close = vm.clone();
    let mut items = stack_panel()
        .spacing(8.0)
        .margin(16.0)
        .child(
            text_block()
                .text("Make the most of your day")
                .font_size(18.0)
                .font_weight(FontWeight::SemiBold)
                .text_wrapping(TextWrapping::Wrap)
                .foreground(p.text),
        )
        .child(
            text_block()
                .text("Here are a few tasks you might want to add to My Day.")
                .font_size(14.0)
                .foreground(p.secondary)
                .text_wrapping(TextWrapping::Wrap)
                .margin((0.0, 0.0, 0.0, 12.0)),
        );
    let suggestions = vm.suggestions();
    if suggestions.is_empty() {
        items = items.child(
            border().margin((0.0, 32.0)).child(
                stack_panel()
                    .spacing(14.0)
                    .child(icon(Icon::Sun, 44.0, p.secondary))
                    .child(
                        text_block()
                            .text("You're all caught up")
                            .font_size(16.0)
                            .foreground(p.text),
                    )
                    .child(
                        text_block()
                            .text("Add tasks to your lists and find suggestions here.")
                            .text_wrapping(TextWrapping::Wrap)
                            .foreground(p.secondary)
                            .font_size(14.0),
                    ),
            ),
        );
    }
    for task in suggestions {
        let id = task.id;
        let add = vm.clone();
        let open = vm.clone();
        items = items.child(
            border()
                .background(p.surface)
                .corner_radius(4.0)
                .padding((10.0, 8.0))
                .child(
                    grid()
                        .column_definitions("*,40")
                        .column_spacing(8.0)
                        .child(
                            button()
                                .name(format!("SuggestionTask{id}"))
                                .class("todo-flat")
                                .automation_properties_name(&task.title)
                                .horizontal_content_alignment(H::Stretch)
                                .background("Transparent")
                                .padding((2.0, 6.0))
                                .on_click(move |_| open.select(Some(id)))
                                .child(
                                    stack_panel()
                                        .spacing(5.0)
                                        .child(
                                            text_block()
                                                .text(&task.title)
                                                .font_size(14.0)
                                                .foreground(p.text)
                                                .text_wrapping(TextWrapping::Wrap),
                                        )
                                        .child(
                                            text_block()
                                                .text(vm.task_metadata(&task))
                                                .font_size(12.0)
                                                .text_trimming(<dyn ferroui_base::media::TextTrimming>::character_ellipsis())
                                                .foreground(p.secondary),
                                        ),
                                ),
                        )
                        .child(border().grid_column(1).vertical_alignment(V::Center).child(
                            todo_controls::toolbar_button(Icon::Add, "Add to My Day", p.accent)
                                .name(format!("SuggestionAdd{id}"))
                                .width(40.0).height(40.0)
                                .on_click(move |event| {
                                    event.set_handled(true);
                                    add.dispatch(Action::SetMyDay { id, day: Some(add.today()) });
                                }),
                        )),
                ),
        );
    }
    border()
        .background(p.background)
        .border_brush(p.border)
        .border_thickness((1.0, 0.0, 0.0, 0.0))
        .child(
            grid()
                .row_definitions("52,*")
                .child(
                    grid()
                        .column_definitions("*,Auto")
                        .margin((16.0, 8.0))
                        .child(
                            stack_panel()
                                .orientation(Orientation::Horizontal)
                                .spacing(10.0)
                                .vertical_alignment(V::Center)
                                .child(icon(Icon::Lightbulb, 20.0, p.accent))
                                .child(
                                    text_block()
                                        .text("Suggestions")
                                        .font_size(16.0)
                                        .font_weight(FontWeight::SemiBold)
                                        .foreground(p.text),
                                ),
                        )
                        .child(border().grid_column(1).child(icon_button(
                            Icon::Close,
                            "Close suggestions",
                            move || close.show_panel(Panel::None),
                        ))),
                )
                .child(scroll_viewer().grid_row(1).child(items)),
        )
        .into()
}
