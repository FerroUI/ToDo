//! The platform's in-process drag source consumes mouse events only. Touch retains native
//! gesture capture and uses the same live reorder/move planners and drop markers instead.
use super::{
    feedback, sidebar_feedback, sidebar_identity,
    touch_plan::{self, Cue, Origin, Plan},
    DragItem,
};
use ferroui_base::input::FocusManager;
use ferroui_base::reactive::Subscription;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{Point, Vector, WeakRef};
use ferroui_controls::{Button, Control, ScrollViewer, TopLevel};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};
use todo_domain::{Id, SidebarItem};
use todo_state::AppViewModel;

pub(crate) struct TouchDrag {
    state: Rc<State>,
    timer: Option<Subscription>,
}

struct State {
    root: WeakRef<TopLevel>,
    vm: Weak<AppViewModel>,
    origin: Origin,
    viewport: Option<WeakRef<ScrollViewer>>,
    point: Cell<Point>,
    last: RefCell<Option<(WeakRef<Control>, Cue)>>,
}

impl TouchDrag {
    pub fn sidebar_available(vm: &AppViewModel, item: SidebarItem) -> bool {
        Origin::capture(vm, DragItem::Sidebar(item)).is_some()
    }

    pub fn new(control: &Control, vm: &Rc<AppViewModel>, id: Id, point: Point) -> Option<Self> {
        Self::start(control, vm, DragItem::Task(id), point)
    }

    pub fn sidebar(
        control: &Control,
        vm: &Rc<AppViewModel>,
        item: SidebarItem,
        point: Point,
    ) -> Option<Self> {
        Self::start(control, vm, DragItem::Sidebar(item), point)
    }

    fn start(
        control: &Control,
        vm: &Rc<AppViewModel>,
        item: DragItem,
        point: Point,
    ) -> Option<Self> {
        let origin = Origin::capture(vm, item)?;
        let state = Rc::new(State {
            root: TopLevel::get_top_level(Some(control))?.downgrade(),
            vm: Rc::downgrade(vm),
            origin,
            viewport: control
                .find_ancestor_of_type::<ScrollViewer>(false)
                .map(|scroll| scroll.downgrade()),
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

    pub fn label(&self) -> &'static str {
        self.state.origin.label()
    }

    pub fn finish(mut self, point: Point) {
        self.timer.take();
        self.state.point.set(point);
        let plan = self.state.plan();
        self.state.clear();
        if let (Some(vm), Some(plan)) = (self.state.vm.upgrade(), plan) {
            vm.dispatch(plan.action);
            if let DragItem::Sidebar(item) = self.state.origin.item {
                let root = self.state.root.clone();
                ferroui_base::threading::Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(root) = root.upgrade() {
                            if let Some(button) =
                                root.find_descendant_of_type_where::<Button>(false, |button| {
                                    sidebar_identity(button) == Some(item)
                                        && FocusManager::can_focus(button)
                                })
                            {
                                button.focus();
                            }
                        }
                    },
                    DispatcherPriority::BACKGROUND,
                );
            }
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
        if let Some((control, cue)) = self
            .last
            .borrow_mut()
            .take()
            .and_then(|(last, cue)| last.upgrade().map(|control| (control, cue)))
        {
            show(&control, cue, false);
        }
    }

    fn plan(&self) -> Option<Plan> {
        let root = self.root.upgrade()?;
        let vm = self.vm.upgrade()?;
        if self.viewport.as_ref().is_some_and(|weak| {
            !weak.upgrade().is_some_and(|viewport| {
                viewport.is_attached_to_visual_tree() && viewport.is_effectively_visible()
            })
        }) {
            return None;
        }
        touch_plan::plan(&root, &vm, &self.origin, self.point.get())
    }

    fn hover(&self, scroll: bool) {
        self.clear();
        let plan = self.plan();
        let viewport = plan
            .as_ref()
            .and_then(|plan| plan.control.find_ancestor_of_type::<ScrollViewer>(false))
            .or_else(|| self.viewport.as_ref().and_then(WeakRef::upgrade));
        if let Some(plan) = plan {
            show(&plan.control, plan.cue, true);
            *self.last.borrow_mut() = Some((plan.control.downgrade(), plan.cue));
        }
        if !scroll {
            return;
        }
        let Some(viewport) = viewport.filter(|viewport| viewport.is_attached_to_visual_tree())
        else {
            return;
        };
        if !self.vm.upgrade().is_some_and(|vm| self.origin.valid(&vm)) {
            return;
        }
        let Some(root) = self.root.upgrade() else {
            return;
        };
        let Some(point) = root.translate_point(self.point.get(), &viewport) else {
            return;
        };
        if point.x < 0.0
            || point.x > viewport.bounds().width
            || point.y < 0.0
            || point.y > viewport.bounds().height
        {
            return;
        }
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

fn show(control: &Control, cue: Cue, active: bool) {
    match cue {
        Cue::Task(after) => feedback::show(control, active, after),
        Cue::Sidebar(placement) => sidebar_feedback::show(control, active.then_some(placement)),
    }
}
