use ferroui_base::reactive::Subscription;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use ferroui_controls::{ListBox, TextBox};
use ferroui_view::prelude::*;
use std::{cell::RefCell, rc::Rc, time::Duration};
use todo_domain::{Action, TaskEntry, View};
use todo_state::AppViewModel;
/// Transient input state has one owner regardless of which responsive pane is mounted.
pub(crate) struct Drafts {
    pub task: Signal<String>,
    pub search: Signal<String>,
    pub task_input: NodeRef<TextBox>,
    pub search_input: NodeRef<TextBox>,
    pub task_list: NodeRef<ListBox>,
    route: RefCell<View>,
    search_timer: RefCell<Option<Subscription>>,
}

impl Drafts {
    pub(super) fn new(view: View) -> Self {
        let query = if let View::Search(query) = &view {
            query.clone()
        } else {
            String::new()
        };
        Self {
            task: signal(String::new()),
            search: signal(query),
            task_input: NodeRef::new(),
            search_input: NodeRef::new(),
            task_list: NodeRef::new(),
            route: RefCell::new(view),
            search_timer: RefCell::new(None),
        }
    }
    pub(super) fn synchronize_route(&self, view: View) {
        if *self.route.borrow() == view {
            return;
        }
        self.search_timer.borrow_mut().take();
        if let View::Search(query) = &view {
            self.search.set(query.clone());
        }
        *self.route.borrow_mut() = view;
    }
    pub fn search_changed(&self, vm: Rc<AppViewModel>, query: String) {
        self.search_timer.borrow_mut().take();
        // The mobile search field stays visible until Return; desktop results update as typed.
        if vm.is_narrow() || vm.view() == View::Search(query.clone()) {
            return;
        }
        *self.search_timer.borrow_mut() = Some(DispatcherTimer::run_once(
            move || vm.navigate(View::Search(query.clone())),
            Duration::from_millis(180),
            DispatcherPriority::BACKGROUND,
        ));
    }
    pub fn submit_search(&self, vm: &AppViewModel) {
        self.search_timer.borrow_mut().take();
        vm.navigate(View::Search(self.search.get_untracked()));
    }
    pub fn submit_task(&self, vm: &AppViewModel) {
        let title = self.task.get_untracked();
        if title.trim().is_empty() {
            return;
        }
        let before = vm.state().tasks.len();
        let (recognize, remove) = {
            let state = vm.state();
            (
                state.settings.recognize_dates,
                state.settings.remove_recognized_dates,
            )
        };
        if recognize {
            let entry = TaskEntry::parse_english(&title, vm.today(), vm.current_minutes(), remove);
            vm.dispatch(Action::AddTaskEntry {
                entry,
                view: vm.view(),
            });
        } else {
            vm.add_task(title);
        }
        if vm.state().tasks.len() > before {
            self.task.set(String::new());
        }
        focus_later(self.task_input, false);
    }
}

pub(super) fn focus_later(node: NodeRef<TextBox>, select_all: bool) {
    Dispatcher::ui_thread().post_local(
        move || {
            if let Some(input) = node
                .get_untracked()
                .filter(|input| input.is_attached_to_visual_tree())
            {
                input.focus();
                if select_all {
                    input.select_all();
                }
            }
        },
        DispatcherPriority::BACKGROUND,
    );
}
