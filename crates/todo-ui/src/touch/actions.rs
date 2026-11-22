use super::{task::TaskRecognizer, task_id};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_controls::Control;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{icon, Icon};
use todo_domain::{Action, Id};
use todo_state::{AppViewModel, Panel};

pub(super) fn view(vm: Rc<AppViewModel>, id: Id) -> Element {
    let (included, can_move, can_delete) =
        vm.state().task(id).map_or((false, false, false), |task| {
            (
                task.is_my_day(vm.today()),
                task.can_move(),
                task.can_delete(),
            )
        });
    let day = vm.clone();
    let move_task = vm.clone();
    let mut right = stack_panel()
        .name("TaskSwipeRightActions")
        .orientation(Orientation::Horizontal)
        .horizontal_alignment(H::Left)
        .is_visible(false)
        .child(action(
            "SwipeMyDay",
            if included {
                "Remove from My Day"
            } else {
                "Add to My Day"
            },
            Icon::Sun,
            "#395EA8",
            move |event| {
                if let Some(id) = close_and_id(event) {
                    let included = day.state().task(id).map(|task| task.is_my_day(day.today()));
                    let Some(included) = included else { return };
                    day.dispatch(Action::SetMyDay {
                        id,
                        day: (!included).then(|| day.today()),
                    });
                }
            },
        ));
    if can_move {
        right = right.child(action(
            "SwipeMove",
            "Move to list",
            Icon::MoveToList,
            "#366F76",
            move |event| {
                if let Some(id) = close_and_id(event) {
                    if move_task
                        .state()
                        .task(id)
                        .is_some_and(|task| task.can_move())
                    {
                        move_task.show_panel(Panel::MoveTask(id));
                    }
                }
            },
        ));
    }
    let mut rails = grid().margin((0.0, 0.0, 0.0, 6.0)).child(right);
    if can_delete {
        rails = rails.child(
            border()
                .name("TaskSwipeDeleteAction")
                .horizontal_alignment(H::Right)
                .is_visible(false)
                .child(action(
                    "SwipeDelete",
                    "Delete",
                    Icon::Trash,
                    "#C42B1C",
                    move |event| {
                        if let Some(id) = close_and_id(event) {
                            // A destructive swipe always asks, independently of the menu preference.
                            if vm.state().task(id).is_some_and(|task| task.can_delete()) {
                                vm.show_panel(Panel::DeleteTask(id));
                            }
                        }
                    },
                )),
        );
    }
    rails.into()
}

fn action(
    name: &str,
    label: &str,
    symbol: Icon,
    color: &str,
    run: impl Fn(&RoutedEventArgs) + 'static,
) -> Element {
    button()
        .name(name)
        .class("todo-flat")
        .width(todo_state::gestures::DELETE_REVEAL)
        .min_height(48.0)
        .padding((4.0, 4.0))
        .vertical_alignment(V::Stretch)
        .background(color)
        .foreground("#FFFFFF")
        .automation_properties_name(label)
        .on_click(move |event| {
            event.set_handled(true);
            run(event);
        })
        .child(
            stack_panel()
                .spacing(2.0)
                .vertical_alignment(V::Center)
                .child(
                    border()
                        .horizontal_alignment(H::Center)
                        .child(icon(symbol, 19.0, "#FFFFFF")),
                )
                .child(
                    text_block()
                        .text(label)
                        .font_size(11.0)
                        .foreground("#FFFFFF")
                        .text_alignment(ferroui_base::media::TextAlignment::Center)
                        .text_wrapping(ferroui_base::media::TextWrapping::Wrap)
                        .max_lines(2),
                ),
        )
        .into()
}

fn close_and_id(event: &RoutedEventArgs) -> Option<Id> {
    let source = event.source()?.cast::<Control>()?;
    for control in source
        .get_self_and_visual_ancestors()
        .filter_map(|visual| visual.cast::<Control>())
    {
        if let Some(id) = task_id(&control) {
            for recognizer in control.gesture_recognizers().to_vec() {
                if let Some(recognizer) = recognizer.cast::<TaskRecognizer>() {
                    recognizer.cancel();
                }
            }
            return Some(id);
        }
    }
    None
}
