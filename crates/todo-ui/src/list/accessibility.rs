use crate::rows::Row;
use ferroui_base::Ref;
use ferroui_controls::{automation::AutomationProperties, Control, ListBox};
use ferroui_view::on_cleanup;

/// The item source contains render projections, whose default boxed name is a
/// Rust type. Name every realized/recycled container from its current item.
pub(super) fn attach(list: &Ref<ListBox>) {
    let prepared = list.container_prepared(|event| update(event.container()));
    let clearing = list.container_clearing(|event| clear(event.container()));
    for container in list.get_realized_containers() {
        update(&container);
    }
    on_cleanup(move || drop((prepared, clearing)));
}

fn update(container: &Control) {
    let item = container.data_context();
    match item.as_ref().and_then(|item| item.downcast_ref::<Row>()) {
        Some(Row::Task {
            title,
            completed,
            important,
            metadata,
            selection_mode,
            multi_selected,
            ..
        }) => {
            AutomationProperties::set_name(container, Some(title));
            AutomationProperties::set_help_text(container, Some(metadata));
            let mut status = vec![if *completed {
                "Completed"
            } else {
                "Not completed"
            }];
            if *important {
                status.push("Important");
            }
            if *selection_mode {
                status.push(if *multi_selected {
                    "Selected"
                } else {
                    "Not selected"
                });
            }
            AutomationProperties::set_item_status(container, Some(&status.join(", ")));
        }
        Some(Row::Header {
            label,
            expanded,
            completed,
            ..
        }) => {
            AutomationProperties::set_name(container, Some(label));
            AutomationProperties::set_help_text(container, None);
            AutomationProperties::set_item_status(
                container,
                if *completed {
                    Some(if *expanded { "Expanded" } else { "Collapsed" })
                } else {
                    None
                },
            );
        }
        None => clear(container),
    }
}

fn clear(container: &Control) {
    AutomationProperties::set_name(container, None);
    AutomationProperties::set_help_text(container, None);
    AutomationProperties::set_item_status(container, None);
}
