use super::*;
use ferroui_base::input::{
    DataFormat, DataTransfer, DataTransferItem, IDataTransfer, KeyModifiers,
};
use ferroui_base::interactivity::RoutedEvent;
use ferroui_base::threading::Dispatcher;
use ferroui_base::Point;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
use ferroui_view::Owner;
use std::cell::Cell;
use todo_domain::AppState;

struct Surface {
    root: Ref<Grid>,
    body: Ref<Control>,
    hint: Ref<Border>,
    vm: Rc<AppViewModel>,
    owner: Option<Owner>,
}

impl Surface {
    fn new() -> Self {
        let owner = Owner::new_root();
        let root = Grid::new();
        root.set_name(Some("TodoApp".into()));
        let body = Grid::new();
        let hint = Border::new();
        hint.set_name(Some(HINT.into()));
        hint.set_is_visible(false);
        body.children().add(hint.clone());
        root.children().add(body.clone());
        let vm = AppViewModel::new(
            AppState::new(),
            "2026-11-23".parse().unwrap(),
            Rc::new(|_| Ok(())),
        );
        owner.with(|| attach_body(&body, vm.clone()));
        Self {
            root,
            body: body.upcast(),
            hint,
            vm,
            owner: Some(owner),
        }
    }

    fn dispose(&mut self) {
        if let Some(owner) = self.owner.take() {
            owner.dispose();
        }
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        self.dispose();
    }
}

fn send(
    target: &Ref<Control>,
    kind: &RoutedEvent<DragEventArgs>,
    data: Rc<dyn IDataTransfer>,
) -> DragEventArgs {
    let event = DragEventArgs::new(
        Some(kind),
        data,
        target.clone().upcast(),
        Point::new(1.0, 1.0),
        KeyModifiers::NONE,
    );
    target.raise_event(&event);
    event
}

fn text_data(read: impl Fn() -> Option<String> + 'static) -> Rc<DataTransfer> {
    let item = DataTransferItem::new();
    item.set_with(&DataFormat::text(), read);
    let data = DataTransfer::new();
    data.add(item);
    data
}

fn plain() -> Rc<DataTransfer> {
    text_data(|| Some("An external task".into()))
}

#[test]
fn host_cancellation_clears_feedback_and_rejects_late_over_and_drop() {
    let _scope = Dispatcher::unit_test_scope();
    let surface = Surface::new();
    let before = surface.vm.state().clone();
    let reads = Rc::new(Cell::new(0));
    let read_count = reads.clone();
    let data = text_data(move || {
        read_count.set(read_count.get() + 1);
        Some("Must not be read after cancellation".into())
    });
    let enter = send(&surface.body, DragDrop::drag_enter_event(), data.clone());
    assert_eq!(enter.drag_effects(), DragDropEffects::COPY);
    assert!(surface.hint.is_visible());
    assert!(crate::drag::has_active(&surface.root));
    assert_eq!(
        reads.get(),
        0,
        "hover inspects metadata without requesting data"
    );

    assert!(crate::cancel_gestures(&surface.root));
    assert!(!surface.hint.is_visible());
    assert!(!crate::drag::has_active(&surface.root));
    assert!(!crate::cancel_gestures(&surface.root));
    let over = send(&surface.body, DragDrop::drag_over_event(), data.clone());
    let dropped = send(&surface.body, DragDrop::drop_event(), data.clone());
    assert_eq!(over.drag_effects(), DragDropEffects::NONE);
    assert_eq!(dropped.drag_effects(), DragDropEffects::NONE);
    assert!(!surface.hint.is_visible());
    assert_eq!(
        reads.get(),
        0,
        "a late Drop must not even request its payload"
    );
    assert_eq!(*surface.vm.state(), before);

    let fresh = send(&surface.body, DragDrop::drag_enter_event(), data.clone());
    assert_eq!(fresh.drag_effects(), DragDropEffects::COPY);
    assert!(
        surface.hint.is_visible(),
        "a new platform enter rearms the target"
    );
    send(&surface.body, DragDrop::drag_leave_event(), data);
    assert!(!surface.hint.is_visible());
    assert!(!crate::drag::has_active(&surface.root));
}

#[test]
fn external_cancellation_is_scoped_to_its_window_root() {
    let _scope = Dispatcher::unit_test_scope();
    let first = Surface::new();
    let second = Surface::new();
    send(&first.body, DragDrop::drag_enter_event(), plain());
    send(&second.body, DragDrop::drag_enter_event(), plain());
    assert!(crate::cancel_gestures(&first.root));
    assert!(!first.hint.is_visible());
    assert!(second.hint.is_visible());
    assert!(crate::drag::has_active(&second.root));
    assert!(crate::cancel_gestures(&second.root));
}

#[test]
fn cancellation_during_native_materialization_invalidates_the_live_drop() {
    let _scope = Dispatcher::unit_test_scope();
    let surface = Surface::new();
    let before = surface.vm.state().clone();
    let root = surface.root.clone();
    let hint = surface.hint.clone();
    let read = Rc::new(Cell::new(false));
    let observed = read.clone();
    let data = text_data(move || {
        observed.set(true);
        assert!(
            !hint.is_visible(),
            "drop removes hover feedback before reading"
        );
        assert!(
            crate::drag::has_active(&root),
            "the read retains its cancellation lease"
        );
        assert!(crate::cancel_gestures(&root));
        Some("No task may survive a canceled native read".into())
    });
    send(&surface.body, DragDrop::drag_enter_event(), data.clone());
    let dropped = send(&surface.body, DragDrop::drop_event(), data);
    assert!(
        read.get(),
        "the provider actually reentered during materialization"
    );
    assert_eq!(dropped.drag_effects(), DragDropEffects::NONE);
    assert!(!crate::drag::has_active(&surface.root));
    assert!(!surface.hint.is_visible());
    assert_eq!(*surface.vm.state(), before);
}

#[test]
fn completing_a_canceled_read_does_not_clear_a_reentrant_new_drag() {
    let _scope = Dispatcher::unit_test_scope();
    let surface = Surface::new();
    let root = surface.root.clone();
    let body = surface.body.clone();
    let data = text_data(move || {
        assert!(crate::cancel_gestures(&root));
        send(&body, DragDrop::drag_enter_event(), plain());
        Some("The old transfer is canceled".into())
    });
    send(&surface.body, DragDrop::drag_enter_event(), data.clone());
    let dropped = send(&surface.body, DragDrop::drop_event(), data);
    assert_eq!(dropped.drag_effects(), DragDropEffects::NONE);
    assert!(crate::drag::has_active(&surface.root));
    assert!(
        surface.hint.is_visible(),
        "old read cleanup leaves fresh feedback intact"
    );
    assert_eq!(
        send(&surface.body, DragDrop::drag_over_event(), plain()).drag_effects(),
        DragDropEffects::COPY
    );
    assert!(crate::cancel_gestures(&surface.root));
}

#[test]
fn disposing_the_drop_surface_invalidates_its_lease_and_removes_handlers() {
    let _scope = Dispatcher::unit_test_scope();
    let mut surface = Surface::new();
    send(&surface.body, DragDrop::drag_enter_event(), plain());
    assert!(surface.hint.is_visible());
    surface.dispose();
    assert!(!surface.hint.is_visible());
    assert!(!crate::drag::has_active(&surface.root));
    let enter = send(&surface.body, DragDrop::drag_enter_event(), plain());
    assert!(!enter.handled());
    assert!(!surface.hint.is_visible());
}

#[test]
fn a_committed_copy_remains_accepted_when_its_notification_cancels_the_lease() {
    let _scope = Dispatcher::unit_test_scope();
    let surface = Surface::new();
    let target = lifetime::Target::new(&surface.body, &surface.vm);
    target.enter();
    let canceled = target.begin_drop().unwrap();
    assert!(crate::cancel_gestures(&surface.root));
    assert_eq!(
        canceled.commit(|| panic!("cancellation must guard before the mutation")),
        Ok(false)
    );

    target.enter();
    let accepted = target.begin_drop().unwrap();
    let notified = Rc::new(Cell::new(false));
    let observed = notified.clone();
    let root = surface.root.clone();
    let vm = Rc::downgrade(&surface.vm);
    let _changed = surface.vm.changed().subscribe_next_guard(move |_| {
        if let Some(vm) = vm.upgrade() {
            if !vm.state().tasks.is_empty() && !observed.replace(true) {
                assert!(crate::cancel_gestures(&root));
            }
        }
    });
    let paste_target = surface.vm.task_paste_target(surface.vm.view());
    let result = accepted.commit(|| {
        surface
            .vm
            .paste_external_tasks(paste_target, None, "Accepted external copy")
    });
    assert_eq!(result, Ok(true));
    assert!(notified.get());
    assert!(
        !accepted.active(),
        "the notification actually canceled the lease"
    );
    assert_eq!(surface.vm.state().tasks.len(), 1);
    assert_eq!(surface.vm.state().tasks[0].title, "Accepted external copy");
}
