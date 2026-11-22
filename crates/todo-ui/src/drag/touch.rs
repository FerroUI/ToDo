//! The platform's in-process drag source consumes mouse events only. Touch retains native
//! gesture capture and uses the same live reorder/move planners and drop markers instead.
use super::{feedback, list_id, move_to, reorder};
use ferroui_base::reactive::Subscription;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{Point, Ref, Vector, WeakRef};
use ferroui_controls::{Control, ScrollViewer, TopLevel};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};
use todo_domain::{Action, Id, SortOrder, View};
use todo_state::{AppViewModel, Panel};

pub(crate) struct TouchDrag {
    state: Rc<State>,
    timer: Option<Subscription>,
}

struct State {
    root: WeakRef<TopLevel>,
    vm: Weak<AppViewModel>,
    id: Id,
    view: View,
    point: Cell<Point>,
    last: RefCell<Option<WeakRef<Control>>>,
}

impl TouchDrag {
    pub fn new(control: &Control, vm: &Rc<AppViewModel>, id: Id, point: Point) -> Option<Self> {
        let View::List(list) = vm.view() else {
            return None;
        };
        if vm.sort() != SortOrder::Manual
            || !vm.state().task(id).is_some_and(|task| task.list_id == list)
        {
            return None;
        }
        let state = Rc::new(State {
            root: TopLevel::get_top_level(Some(control))?.downgrade(),
            vm: Rc::downgrade(vm),
            id,
            view: vm.view(),
            point: Cell::new(point),
            last: RefCell::new(None),
        });
        let weak = Rc::downgrade(&state);
        let timer = DispatcherTimer::run(
            move || {
                let Some(state) = weak.upgrade() else {
                    return false;
                };
                state.hover(true);
                true
            },
            Duration::from_millis(45),
            DispatcherPriority::BACKGROUND,
        );
        state.hover(false);
        Some(Self {
            state,
            timer: Some(timer),
        })
    }

    pub fn update(&self, point: Point) {
        self.state.point.set(point);
        self.state.hover(false);
    }

    pub fn finish(mut self, point: Point) {
        self.timer.take();
        self.state.point.set(point);
        let plan = self.state.plan();
        self.state.clear();
        if let (Some(vm), Some((_, action, _))) = (self.state.vm.upgrade(), plan) {
            vm.dispatch(action);
        }
    }
}

impl Drop for TouchDrag {
    fn drop(&mut self) {
        self.timer.take();
        self.state.clear();
    }
}

impl State {
    fn clear(&self) {
        if let Some(control) = self
            .last
            .borrow_mut()
            .take()
            .and_then(|last| last.upgrade())
        {
            feedback::show(&control, false, false);
        }
    }

    fn plan(&self) -> Option<(Ref<Control>, Action, bool)> {
        let root = self.root.upgrade()?;
        let vm = self.vm.upgrade()?;
        let View::List(source_list) = self.view else {
            return None;
        };
        if vm.view() != self.view
            || vm.sort() != SortOrder::Manual
            || vm.panel() != Panel::None
            || vm.rename_session().is_some()
            || !vm
                .state()
                .task(self.id)
                .is_some_and(|task| task.list_id == source_list)
        {
            return None;
        }
        let hit = root.input_hit_test_with(self.point.get(), false)?;
        for control in hit
            .get_self_and_visual_ancestors()
            .filter_map(|visual| visual.cast::<Control>())
        {
            let point = root.translate_point(self.point.get(), &control)?;
            let after = point.y >= control.bounds().height / 2.0;
            if let Some(target) = crate::touch::task_id(&control) {
                return reorder(&vm, self.id, target, after).map(|action| (control, action, after));
            }
            if let Some(list) = list_id(&control) {
                return move_to(&vm, self.id, list).map(|action| (control, action, false));
            }
        }
        None
    }

    fn hover(&self, scroll: bool) {
        self.clear();
        let Some((control, _, after)) = self.plan() else {
            return;
        };
        feedback::show(&control, true, after);
        *self.last.borrow_mut() = Some(control.downgrade());
        if !scroll {
            return;
        }
        let Some(viewport) = control.find_ancestor_of_type::<ScrollViewer>(false) else {
            return;
        };
        let Some(root) = self.root.upgrade() else {
            return;
        };
        let Some(point) = root.translate_point(self.point.get(), &viewport) else {
            return;
        };
        let delta = if point.y < 32.0 {
            -12.0
        } else if point.y > viewport.bounds().height - 32.0 {
            12.0
        } else {
            0.0
        };
        let offset = viewport.offset();
        let maximum = (viewport.extent().height - viewport.viewport().height).max(0.0);
        viewport.set_offset(Vector::new(
            offset.x,
            (offset.y + delta).clamp(0.0, maximum),
        ));
    }
}
