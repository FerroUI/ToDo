use crate::palette;
use ferroui_base::input::Key;
use ferroui_controls::{Control, TextBox};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{inherited_icon, Icon};
use todo_domain::{Action, Id};
use todo_state::AppViewModel;

pub(super) fn view(vm: Rc<AppViewModel>, id: Id) -> Element {
    let p = palette(&vm);
    let add = vm.clone();
    let click = vm.clone();
    let draft = vm.clone();
    grid().key("new-step").column_definitions("40,*").margin((0.0, 2.0))
        .child(button().class("todo-toolbar").width(40.0).height(40.0).padding(12.0)
            .foreground(p.accent).automation_properties_name("Add step")
            .tool_tip_tip("Add step").child(inherited_icon(Icon::Add, 16.0))
            .on_click(move |event| {
                let field = event.source().and_then(|source| source.cast::<Control>()).and_then(|button| {
                    button.get_visual_ancestors().filter_map(|parent| parent.cast::<Control>())
                        .find_map(|parent| parent.find_descendant_of_type_where::<TextBox>(false, |field| field.name().as_deref() == Some("NewStep")))
                });
                if let Some(field) = field { submit(&click, id, &field); }
                event.set_handled(true);
            }))
        .child(text_box().grid_column(1).name("NewStep").text(vm.step_draft(id))
            .class("todo-input").placeholder_text("Add step").font_size(14.0)
            .automation_properties_name("Add step")
            .automation_properties_help_text("Type a step and press Enter. You can keep adding steps without leaving this field.")
            .foreground(p.accent).background("Transparent").border_thickness(0.0)
            .on_changed(TextBox::text_property(), move |text| draft.set_step_draft(id, text.clone().unwrap_or_default()))
            .on_key_down(move |event| {
                if event.key == Key::Return && !crate::editing::is_composing(event) {
                    if let Some(field) = event.source().and_then(|source| source.cast::<TextBox>()) { submit(&add, id, &field); }
                    event.set_handled(true);
                }
            })).into()
}

fn submit(vm: &AppViewModel, id: Id, field: &TextBox) {
    let title = field.text().unwrap_or_default();
    if !title.trim().is_empty() {
        let before = vm.state().task(id).map_or(0, |task| task.steps.len());
        vm.dispatch(Action::AddStep { task_id: id, title });
        if vm
            .state()
            .task(id)
            .is_some_and(|task| task.steps.len() > before)
        {
            vm.set_step_draft(id, String::new());
            field.set_text(Some(""));
        }
    }
    field.focus();
}
