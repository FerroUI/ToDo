use super::item;
use ferroui_controls::MenuItemToggleType;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_domain::SortOrder;
use todo_state::AppViewModel;

pub(super) fn items(vm: Rc<AppViewModel>) -> Vec<Element> {
    [
        SortOrder::Manual,
        SortOrder::Importance,
        SortOrder::DueDate,
        SortOrder::Alphabetical,
        SortOrder::Created,
        SortOrder::MyDay,
    ]
    .into_iter()
    .map(|order| {
        let choose = vm.clone();
        menu_item()
            .header(order.label())
            .automation_properties_name(order.label())
            .toggle_type(MenuItemToggleType::Radio)
            .group_name("TaskSort")
            .is_checked(vm.sort() == order)
            .on_key_down(super::keys::isolate)
            .on_click(move |event| {
                item::close(event);
                choose.set_sort(order);
                event.set_handled(true);
            })
            .into()
    })
    .collect()
}
