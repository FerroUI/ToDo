use super::*;
use todo_domain::AppState;

fn fixture() -> (
    Rc<AppViewModel>,
    NativeServices,
    std::sync::mpsc::Receiver<Request>,
) {
    let vm = AppViewModel::new(
        AppState::new(),
        "2026-11-23".parse().unwrap(),
        Rc::new(|_| Ok(())),
    );
    vm.add_task("Retain this local task".into());
    vm.set_cloud_capabilities(true, true, false);
    vm.set_cloud_state(true, "Test account");
    let (transport, requests, _) = Transport::controlled();
    let native = NativeServices {
        transport,
        queue: RefCell::new(VecDeque::new()),
        epoch: Cell::new(0),
        busy: Cell::new(false),
        available: Cell::new(true),
        checked: Cell::new(true),
        runtime_error: RefCell::new(None),
        paused: Cell::new(false),
        failures: Cell::new(0),
        next_sync: Cell::new(Instant::now()),
        changed: Cell::new(None),
        revision: Cell::new(vm.revision()),
        document: RefCell::new(vm.export_json().unwrap()),
        demo: false,
        wake_id: 0,
    };
    (vm, native, requests)
}

#[test]
fn disconnected_transport_releases_refresh_and_preserves_local_tasks() {
    let (vm, native, requests) = fixture();
    let document = vm.export_json().unwrap();
    drop(requests);
    vm.refresh();
    assert!(vm.refreshing());
    let error = native
        .submit(&vm, json!({"kind": "syncMicrosoft"}))
        .unwrap_err();
    assert!(error.contains("connection closed"));
    assert!(
        !vm.refreshing(),
        "a failed send has no completion event to finish refresh"
    );
    assert!(
        !vm.cloud_connected(),
        "a stopped actor cannot retain a live UI session"
    );
    assert!(!native.busy.get());
    assert!(native.queue.borrow().is_empty());
    assert_eq!(vm.export_json().unwrap(), document);
    vm.refresh();
    assert!(vm.refreshing(), "refresh can start again after the error");
    assert_eq!(
        native
            .submit(&vm, json!({"kind": "syncMicrosoft"}))
            .unwrap_err(),
        error
    );
    assert!(!vm.refreshing());
}

#[test]
fn transport_failure_while_draining_queued_refresh_releases_its_ui_state() {
    let (vm, native, requests) = fixture();
    native.submit(&vm, json!({"kind": "initialize"})).unwrap();
    assert_eq!(requests.recv().unwrap().action["kind"], "initialize");
    vm.refresh();
    native
        .submit(&vm, json!({"kind": "syncMicrosoft"}))
        .unwrap();
    native
        .submit(&vm, json!({"kind": "syncMicrosoft"}))
        .unwrap();
    assert_eq!(
        native.queue.borrow().len(),
        1,
        "manual refresh requests are coalesced"
    );
    vm.add_task("Edited while waiting".into());
    let document = vm.export_json().unwrap();
    drop(requests);
    native.busy.set(false); // The preceding command completed before the actor stopped.
    native.dispatch_next(&vm);
    assert!(!vm.refreshing());
    assert!(!vm.cloud_connected());
    assert!(!native.available.get());
    assert!(native.queue.borrow().is_empty());
    assert_eq!(vm.export_json().unwrap(), document);
}

#[test]
fn accepted_sync_keeps_refresh_active_until_a_real_worker_completion() {
    let (vm, native, requests) = fixture();
    vm.refresh();
    native
        .submit(&vm, json!({"kind": "syncMicrosoft"}))
        .unwrap();
    let request = requests.recv().unwrap();
    assert_eq!(request.action["kind"], "syncMicrosoft");
    assert_eq!(request.generation, native.epoch.get());
    assert!(vm.refreshing());
    assert!(vm.cloud_connected());
    assert!(native.busy.get());
}
