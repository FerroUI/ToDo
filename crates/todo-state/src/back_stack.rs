use crate::AppViewModel;
use todo_domain::View;

impl AppViewModel {
    pub(crate) fn remember_navigation(&self, next: &View) {
        let current = self.view();
        // Search typing is one navigation, and closing search must not create a
        // loop that reopens the dismissed query on the next Back gesture.
        if current == *next || matches!(current, View::Search(_)) {
            return;
        }
        let mut history = self.navigation_history.borrow_mut();
        if history.last() != Some(&current) {
            history.push(current);
            if history.len() > 64 {
                history.remove(0);
            }
        }
    }

    pub fn can_go_back_view(&self) -> bool {
        let current = self.view();
        self.navigation_history
            .borrow()
            .iter()
            .any(|view| *view != current && self.valid_back_view(view))
    }

    pub fn go_back_view(&self) -> bool {
        loop {
            let previous = self.navigation_history.borrow_mut().pop();
            let Some(previous) = previous else {
                return false;
            };
            if self.valid_back_view(&previous) && previous != self.view() {
                self.navigate_core(previous);
                return true;
            }
        }
    }

    pub fn return_from_search(&self, origin: View) {
        if self.navigation_history.borrow().last() == Some(&origin) {
            self.navigation_history.borrow_mut().pop();
        }
        self.navigate_core(if self.valid_back_view(&origin) {
            origin
        } else {
            View::MyDay
        });
    }

    fn valid_back_view(&self, view: &View) -> bool {
        !matches!(view, View::Search(_))
            && !matches!(view, View::List(id) if self.state().list(*id).is_none())
    }
}
