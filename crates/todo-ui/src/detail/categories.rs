use super::{action_row::clear_button, fields::section};
use crate::palette;
use ferroui_base::input::Key;
use ferroui_base::layout::VerticalAlignment as V;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::{Control, TextBox};
use ferroui_view::prelude::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use todo_controls::{inherited_icon, Icon};
use todo_domain::{Action, Id, Task};
use todo_state::AppViewModel;

type Drafts = Rc<RefCell<BTreeMap<Id, String>>>;

pub(super) fn view(vm: Rc<AppViewModel>, task: &Task) -> Element {
    let id = task.id;
    rerender(move || content(vm.clone(), id))
}

fn content(vm: Rc<AppViewModel>, id: Id) -> Element {
    let drafts = use_hook(|| Rc::new(RefCell::new(BTreeMap::<Id, String>::new())));
    let Some(task) = vm.state().task(id).cloned() else {
        return Element::Empty;
    };
    let p = palette(&vm);
    let mut content = stack_panel().spacing(2.0).child(
        text_block()
            .text("Categories")
            .font_size(12.0)
            .font_weight(FontWeight::SemiBold)
            .foreground(p.secondary)
            .margin((16.0, 12.0, 12.0, 4.0)),
    );
    for category in &task.tags {
        let name = category.clone();
        let remove = vm.clone();
        content =
            content.child(
                grid()
                    .column_definitions("*,40")
                    .min_height(40.0)
                    .child(
                        text_block()
                            .text(category)
                            .font_size(14.0)
                            .foreground(p.text)
                            .text_wrapping(TextWrapping::Wrap)
                            .vertical_alignment(V::Center)
                            .margin((16.0, 6.0, 4.0, 6.0)),
                    )
                    .child(border().grid_column(1).vertical_alignment(V::Center).child(
                        clear_button(
                            &format!("Remove category {category}"),
                            p.secondary,
                            "AddCategory",
                            move || {
                                let tags = remove
                                    .state()
                                    .task(id)
                                    .map(|task| {
                                        task.tags
                                            .iter()
                                            .filter(|tag| tag.as_str() != name.as_str())
                                            .cloned()
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                remove.dispatch(Action::SetTags { id, tags });
                            },
                        ),
                    )),
            );
    }
    let enter = vm.clone();
    let click = vm.clone();
    let enter_drafts = drafts.clone();
    let click_drafts = drafts.clone();
    let track = drafts.clone();
    let draft = drafts.borrow().get(&id).cloned().unwrap_or_default();
    content = content.child(
        grid()
            .column_definitions("*,40")
            .margin((10.0, 2.0, 0.0, 6.0))
            .child(
                text_box()
                    .name("NewCategory")
                    .text(draft)
                    .class("todo-input")
                    .placeholder_text("Add a category")
                    .automation_properties_name("New category")
                    .automation_properties_help_text(
                        "Enter one category name and press Enter. Commas are part of the name.",
                    )
                    .font_size(14.0)
                    .min_height(40.0)
                    .padding((6.0, 8.0))
                    .vertical_content_alignment(V::Center)
                    .foreground(p.text)
                    .background("Transparent")
                    .border_thickness(0.0)
                    .on_changed(TextBox::text_property(), move |value| {
                        track
                            .borrow_mut()
                            .insert(id, value.clone().unwrap_or_default());
                    })
                    .on_key_down(move |event| {
                        if event.key == Key::Return && !crate::editing::is_composing(event) {
                            if let Some(field) =
                                event.source().and_then(|source| source.cast::<TextBox>())
                            {
                                submit(&enter, id, &field, &enter_drafts);
                            }
                            event.set_handled(true);
                        }
                    }),
            )
            .child(
                button()
                    .grid_column(1)
                    .name("AddCategory")
                    .class("todo-toolbar")
                    .width(40.0)
                    .height(40.0)
                    .padding(12.0)
                    .foreground(p.accent)
                    .automation_properties_name("Add category")
                    .tool_tip_tip("Add category")
                    .child(inherited_icon(Icon::Add, 16.0))
                    .on_click(move |event| {
                        let field = event
                            .source()
                            .and_then(|source| source.cast::<Control>())
                            .and_then(|button| {
                                button
                                    .get_visual_ancestors()
                                    .filter_map(|parent| parent.cast::<Control>())
                                    .find_map(|parent| {
                                        parent.find_descendant_of_type_where::<TextBox>(
                                            false,
                                            |field| field.name().as_deref() == Some("NewCategory"),
                                        )
                                    })
                            });
                        if let Some(field) = field {
                            submit(&click, id, &field, &click_drafts);
                        }
                        event.set_handled(true);
                    }),
            ),
    );
    section(vm, content)
}

fn submit(vm: &AppViewModel, id: Id, field: &TextBox, drafts: &Drafts) {
    let value = field.text().unwrap_or_default().trim().to_owned();
    if value.is_empty() {
        field.focus();
        return;
    }
    let Some(mut tags) = vm.state().task(id).map(|task| task.tags.clone()) else {
        return;
    };
    if !tags.iter().any(|name| name.eq_ignore_ascii_case(&value)) {
        tags.push(value);
        drafts.borrow_mut().remove(&id);
        vm.dispatch(Action::SetTags { id, tags });
    }
    drafts.borrow_mut().remove(&id);
    field.set_text(Some(""));
    field.focus();
}
