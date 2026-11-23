use crate::palette;
use ferroui_base::input::Key;
use ferroui_base::media::TextWrapping;
use ferroui_controls::automation::AutomationLiveSetting;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{action_button, primary_button, text_button, Icon};
use todo_state::{AppViewModel, HostRequest};

const COLORS: [(&str, &str); 26] = [
    ("No color", "#8A8886"),
    ("Red", "#D13438"),
    ("Orange", "#F7630C"),
    ("Brown", "#8E562E"),
    ("Yellow", "#FFB900"),
    ("Green", "#498205"),
    ("Teal", "#038387"),
    ("Olive", "#8C8C14"),
    ("Blue", "#0078D4"),
    ("Purple", "#8764B8"),
    ("Cranberry", "#C239B3"),
    ("Steel", "#567C73"),
    ("Dark steel", "#365D54"),
    ("Gray", "#8A8886"),
    ("Dark gray", "#605E5C"),
    ("Black", "#323130"),
    ("Dark red", "#A4262C"),
    ("Dark orange", "#CA5010"),
    ("Dark brown", "#603D30"),
    ("Dark yellow", "#986F0B"),
    ("Dark green", "#0B6A0B"),
    ("Dark teal", "#004B50"),
    ("Dark olive", "#6B6B00"),
    ("Dark blue", "#004E8C"),
    ("Dark purple", "#5C2D91"),
    ("Dark cranberry", "#881798"),
];

fn preset(index: usize) -> String {
    if index == 0 {
        "none".into()
    } else {
        format!("preset{}", index - 1)
    }
}

fn color_picker(
    vm: Rc<AppViewModel>,
    label: String,
    current: String,
    choose: Rc<dyn Fn(String)>,
) -> Element {
    let selected = COLORS
        .iter()
        .enumerate()
        .find(|(index, _)| preset(*index) == current)
        .map_or("No color", |(_, (name, _))| *name);
    button()
        .class("todo-action")
        .min_height(40.0)
        .key(format!("{label}:{current}"))
        .automation_properties_name(format!("{label}: {selected}"))
        .child(
            text_block()
                .text(format!("{label}: {selected}"))
                .font_size(14.0)
                .text_wrapping(TextWrapping::Wrap),
        )
        .with(move |button| {
            let current = current.clone();
            let choose = choose.clone();
            crate::menus::attach_actions(button, vm.clone(), move || {
                COLORS
                    .iter()
                    .enumerate()
                    .map(|(index, (name, color))| {
                        let value = preset(index);
                        let choose = choose.clone();
                        crate::menus::color_command(name, color, current == value, move || {
                            choose(value.clone())
                        })
                    })
                    .collect()
            });
        })
        .into()
}

pub(super) fn view(vm: Rc<AppViewModel>) -> Element {
    rerender(move || {
        let expanded = use_signal(|| false);
        let name = use_signal(String::new);
        let color = use_signal(|| "preset7".to_owned());
        let error = use_signal(String::new);
        let deleting = use_signal(|| None::<String>);
        let p = palette(&vm);
        let mut content = stack_panel().spacing(8.0).child(action_button(
            Icon::List,
            if expanded.get() {
                "Close category management"
            } else {
                "Manage Outlook categories"
            },
            move || expanded.set(!expanded.get()),
        ));
        if !expanded.get() {
            return content.into_element();
        }
        if !vm.state().settings.outlook_categories_write_enabled {
            let enable = vm.clone();
            return content.child(text_block().text("Create categories, change their colors, or remove them from your Outlook catalogue. Enabling editing requests permission to update your mailbox settings.")
                    .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
                .child(action_button(Icon::Settings, "Enable category editing", move || enable.request(HostRequest::EnableOutlookCategoryEditing)))
                .into_element();
        }
        let create = vm.clone();
        let submit: Rc<dyn Fn()> = Rc::new(move || {
            let value = name.get().trim().to_owned();
            if value.is_empty()
                || value.chars().count() > 255
                || value.chars().any(char::is_control)
            {
                error.set("Enter a category name of 1–255 characters without line breaks.".into());
                return;
            }
            if create
                .outlook_categories()
                .iter()
                .any(|category| category.name.to_lowercase() == value.to_lowercase())
            {
                error.set("An Outlook category with this name already exists.".into());
                return;
            }
            error.set(String::new());
            // Retain the draft until the service confirms the write. A network
            // or consent error must never discard what the user entered.
            create.request(HostRequest::CreateOutlookCategory {
                name: value,
                color: color.get(),
            });
        });
        let enter = submit.clone();
        content = content
            .child(
                text_box()
                    .name("OutlookCategoryName")
                    .text(name.two_way())
                    .max_length(510)
                    .automation_properties_name("New Outlook category name")
                    .placeholder_text("Category name")
                    .font_size(14.0)
                    .min_height(40.0)
                    .on_key_down(move |event| {
                        if event.key == Key::Return && !crate::editing::is_composing(event) {
                            enter();
                            event.set_handled(true);
                        }
                    }),
            )
            .child(color_picker(
                vm.clone(),
                "New category color".into(),
                color.get(),
                Rc::new(move |value| color.set(value)),
            ))
            .child(
                text_block()
                    .text(error.get())
                    .is_visible(!error.get().is_empty())
                    .font_size(12.0)
                    .foreground(if vm.is_dark() { "#F1707B" } else { "#D13438" })
                    .text_wrapping(TextWrapping::Wrap)
                    .automation_properties_live_setting(AutomationLiveSetting::Assertive),
            )
            .child(primary_button("Create Outlook category", move || submit()));
        for category in vm.outlook_categories() {
            let id = category.id.clone();
            let update = vm.clone();
            let expected = if category.preset.is_empty() {
                "none".into()
            } else {
                category.preset.clone()
            };
            let current = expected.clone();
            let remove_id = id.clone();
            let mut row = stack_panel()
                .spacing(4.0)
                .margin((0.0, 8.0))
                .child(
                    text_block()
                        .text(&category.name)
                        .font_size(14.0)
                        .foreground(p.text)
                        .text_wrapping(TextWrapping::Wrap),
                )
                .child(color_picker(
                    vm.clone(),
                    format!("Color for {}", category.name),
                    current,
                    Rc::new(move |color| {
                        update.request(HostRequest::UpdateOutlookCategory {
                            id: id.clone(),
                            color,
                            expected_color: expected.clone(),
                        });
                    }),
                ))
                .child(text_button(
                    &format!("Remove category {} from Outlook", category.name),
                    move || deleting.set(Some(remove_id.clone())),
                ));
            if deleting.get().as_ref() == Some(&category.id) {
                let remove = vm.clone();
                let id = category.id;
                row = row.child(text_block().text("Remove this category from the Outlook catalogue? Category names already assigned to tasks are kept.")
                        .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap))
                    .child(text_button("Cancel category removal", move || deleting.set(None)))
                    .child(text_button("Confirm category removal", move || {
                        remove.request(HostRequest::DeleteOutlookCategory { id: id.clone() });
                        deleting.set(None);
                    }));
            }
            content = content.child(row);
        }
        let disable = vm.clone();
        content
            .child(action_button(
                Icon::Close,
                "Turn off category editing",
                move || disable.request(HostRequest::DisableOutlookCategoryEditing),
            ))
            .into_element()
    })
}
