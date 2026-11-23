use crate::palette;
use ferroui_base::input::Key;
use ferroui_base::media::TextWrapping;
use ferroui_controls::TextBox;
use ferroui_view::prelude::*;
use std::{collections::BTreeMap, rc::Rc};
use todo_domain::Id;
use todo_state::{AppViewModel, Panel};

pub(super) fn view(vm: Rc<AppViewModel>, ids: Vec<Id>) -> Element {
    rerender(move || {
        let p = palette(&vm);
        let name = use_signal(String::new);
        let error = use_signal(String::new);
        let state = vm.state();
        let tasks: Vec<_> = ids.iter().filter_map(|id| state.task(*id)).collect();
        let mut names = BTreeMap::new();
        for category in vm.outlook_categories() {
            names.insert(
                category.name.to_lowercase(),
                (category.name, category.color),
            );
        }
        for task in &state.tasks {
            for tag in &task.tags {
                names
                    .entry(tag.to_lowercase())
                    .or_insert_with(|| (tag.clone(), "#8A8886".into()));
            }
        }
        let mut content = stack_panel().spacing(8.0).child(
            text_block()
                .text(format!("Apply categories to {} selected tasks.", ids.len()))
                .text_wrapping(TextWrapping::Wrap)
                .foreground(p.secondary),
        );
        for (_, (label, color)) in names {
            let checked = !tasks.is_empty()
                && tasks
                    .iter()
                    .all(|task| task.tags.iter().any(|tag| tag.eq_ignore_ascii_case(&label)));
            let partial = !checked
                && tasks
                    .iter()
                    .any(|task| task.tags.iter().any(|tag| tag.eq_ignore_ascii_case(&label)));
            let change = vm.clone();
            let targets = ids.clone();
            let category = label.clone();
            content = content.child(
                check_box()
                    .is_three_state(false)
                    .is_checked(Some(checked))
                    .automation_properties_name(format!(
                        "{} category {label}{}",
                        if checked { "Remove" } else { "Add" },
                        if partial {
                            " (some selected tasks)"
                        } else {
                            ""
                        }
                    ))
                    .child(
                        stack_panel()
                            .orientation(ferroui_base::layout::Orientation::Horizontal)
                            .spacing(8.0)
                            .child(
                                border()
                                    .width(12.0)
                                    .height(12.0)
                                    .background(color.as_str())
                                    .corner_radius(3.0),
                            )
                            .child(
                                text_block()
                                    .text(label)
                                    .text_wrapping(TextWrapping::Wrap)
                                    .foreground(p.text),
                            ),
                    )
                    .on_click(move |_| {
                        if let Err(message) =
                            change.set_tasks_category(&targets, &category, !checked)
                        {
                            error.set(message);
                        }
                    }),
            );
        }
        drop(state);
        let add = vm.clone();
        let targets = ids.clone();
        let submit: Rc<dyn Fn()> =
            Rc::new(
                move || match add.set_tasks_category(&targets, &name.get(), true) {
                    Ok(()) => {
                        name.set(String::new());
                        error.set(String::new());
                    }
                    Err(message) => error.set(message),
                },
            );
        let enter = submit.clone();
        let close = vm.clone();
        content
            .child(
                text_box()
                    .name("BulkCategoryName")
                    .automation_properties_name("Category for selected tasks")
                    .placeholder_text("Add a category")
                    .text(name.two_way())
                    .on_key_down(move |event| {
                        if event.key == Key::Return && !crate::editing::is_composing(event) {
                            enter();
                            event.set_handled(true);
                        }
                    })
                    .on_changed(TextBox::text_property(), move |_| error.set(String::new())),
            )
            .child(
                text_block()
                    .text(error.get())
                    .foreground(if vm.is_dark() { "#F1707B" } else { "#D13438" })
                    .text_wrapping(TextWrapping::Wrap),
            )
            .child(todo_controls::primary_button("Add category", move || {
                submit()
            }))
            .child(todo_controls::text_button("Done", move || {
                close.show_panel(Panel::None)
            }))
            .into_element()
    })
}
