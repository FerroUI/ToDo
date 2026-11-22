use super::counts::Counts;
use crate::palette;
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{icon, Icon};
use todo_domain::View;
use todo_state::AppViewModel;
pub(super) fn view(
    vm: Rc<AppViewModel>,
    target: View,
    symbol: Icon,
    color: &str,
    counts: &Counts,
) -> Element {
    let p = palette(&vm);
    let selected = vm.view() == target;
    let title = target.title(&vm.state());
    let count = counts.get(&target);
    let go = vm.clone();
    let drop_target = vm.clone();
    let context = vm.clone();
    let list_id = match target {
        View::List(id) => Some(id),
        _ => None,
    };
    button()
        .key(format!("nav-{target:?}"))
        .automation_properties_name(title.clone())
        .tag(crate::drag::list_tag(&target))
        .with(move |button| {
            crate::drag::attach_list_target(button, drop_target.clone());
            if let Some(id) = list_id {
                crate::menus::attach_list(button, context.clone(), id);
            }
        })
        .class(if selected {
            "todo-nav selected"
        } else {
            "todo-nav"
        })
        .height(44.0)
        .padding((12.0, 0.0))
        .horizontal_content_alignment(H::Stretch)
        .on_click(move |_| go.navigate(target.clone()))
        .child(
            grid()
                .column_definitions("28,*,Auto")
                .child(
                    border()
                        .vertical_alignment(V::Center)
                        .child(icon(symbol, 20.0, color)),
                )
                .child(
                    text_block()
                        .grid_column(1)
                        .text(title)
                        .font_size(14.0)
                        .foreground(if selected { p.accent } else { p.text })
                        .text_trimming(<dyn ferroui_base::media::TextTrimming>::character_ellipsis())
                        .font_weight(if selected {
                            FontWeight::SemiBold
                        } else {
                            FontWeight::Normal
                        })
                        .vertical_alignment(V::Center),
                )
                .child(
                    text_block()
                        .grid_column(2)
                        .text(if count > 0 {
                            count.to_string()
                        } else {
                            String::new()
                        })
                        .font_size(12.0)
                        .foreground(p.secondary)
                        .vertical_alignment(V::Center),
                ),
        )
        .into()
}
