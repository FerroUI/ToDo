use ferroui_base::{
    threading::{DispatcherPriority, DispatcherTimer},
    Ref,
};
use ferroui_controls::{
    platform::PlatformManager, NativeMenu, NativeMenuItem, Window, WindowCloseReason, WindowIcon,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use todo_state::{AppViewModel, HostRequest};

pub fn install(vm: &Rc<AppViewModel>, window: &Ref<Window>) {
    let Some(tray) = PlatformManager::create_tray_icon() else {
        return;
    };
    let Some(exporter) = tray.menu_exporter() else {
        tray.dispose();
        return;
    };
    let Ok(icon) = WindowIcon::from_stream(&mut std::io::Cursor::new(include_bytes!(
        "../../assets/icon.png"
    ))) else {
        tray.dispose();
        return;
    };
    tray.set_icon(Some(icon.platform_impl()));
    tray.set_is_visible(false);
    tray.set_tool_tip_text(Some("To Do"));
    let menu = NativeMenu::new();
    let open = NativeMenuItem::new();
    open.set_header(Some("Open To Do".into()));
    let sync = NativeMenuItem::new();
    sync.set_header(Some("Sync now".into()));
    let quit = NativeMenuItem::new();
    quit.set_header(Some("Quit To Do".into()));
    menu.add(open.clone());
    menu.add(sync.clone());
    menu.add(quit.clone());
    exporter.set_native_menu(Some(menu));
    let quitting = Rc::new(Cell::new(false));
    let weak = window.downgrade();
    open.click(move |_| {
        if let Some(window) = weak.upgrade() {
            window.show();
            window.activate();
        }
    })
    .detach();
    let weak = window.downgrade();
    tray.set_on_clicked(Some(Rc::new(move || {
        if let Some(window) = weak.upgrade() {
            window.show();
            window.activate();
        }
    })));
    let weak = Rc::downgrade(vm);
    sync.click(move |_| {
        if let Some(vm) = weak.upgrade() {
            vm.request(HostRequest::SyncMicrosoft);
        }
    })
    .detach();
    let weak = window.downgrade();
    let explicit_quit = quitting.clone();
    quit.click(move |_| {
        explicit_quit.set(true);
        if let Some(window) = weak.upgrade() {
            window.close();
        }
    })
    .detach();
    let present = Arc::new(AtomicBool::new(!cfg!(target_os = "linux")));
    let stopped = Arc::new(AtomicBool::new(false));
    probe(present.clone(), stopped.clone());
    let weak_vm = Rc::downgrade(vm);
    let weak_window = window.downgrade();
    let supported = present.clone();
    window
        .closing(move |event| {
            if quitting.get()
                || !matches!(
                    event.close_reason(),
                    WindowCloseReason::WindowClosing | WindowCloseReason::Undefined
                )
            {
                return;
            }
            if let (Some(vm), Some(window)) = (weak_vm.upgrade(), weak_window.upgrade()) {
                if supported.load(Ordering::Relaxed)
                    && vm.state().settings.keep_running_in_background
                {
                    todo_ui::commit_pending_edits(&vm, &window);
                    event.set_cancel(true);
                    window.hide();
                }
            }
        })
        .detach();
    let weak_vm = Rc::downgrade(vm);
    let weak_window = window.downgrade();
    let live_tray = tray.clone();
    let timer = DispatcherTimer::run(
        move || {
            let (Some(vm), Some(window)) = (weak_vm.upgrade(), weak_window.upgrade()) else {
                return false;
            };
            let supported = present.load(Ordering::Relaxed);
            vm.set_background_supported(supported);
            let enabled = supported && vm.state().settings.keep_running_in_background;
            live_tray.set_is_visible(enabled);
            sync.set_is_enabled(vm.cloud_connected());
            if !enabled && !window.is_visible() {
                window.show();
                window.activate();
            }
            true
        },
        Duration::from_secs(1),
        DispatcherPriority::BACKGROUND,
    );
    let timer = RefCell::new(Some(timer));
    window
        .closed(move || {
            stopped.store(true, Ordering::Relaxed);
            tray.dispose();
            if let Some(timer) = timer.borrow_mut().take() {
                timer.dispose();
            }
        })
        .detach();
}

#[cfg(target_os = "linux")]
fn probe(present: Arc<AtomicBool>, stopped: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        // A session bus alone is insufficient: GNOME without an indicator host
        // must not hide the only window behind an icon nobody can reach.
        let connection = zbus::block_on(zbus::Connection::session()).ok();
        while !stopped.load(Ordering::Relaxed) {
            let available = connection.as_ref().is_some_and(|connection| {
                zbus::block_on(async {
                    let proxy = zbus::Proxy::new(
                        connection,
                        "org.kde.StatusNotifierWatcher",
                        "/StatusNotifierWatcher",
                        "org.kde.StatusNotifierWatcher",
                    )
                    .await?;
                    proxy
                        .get_property::<bool>("IsStatusNotifierHostRegistered")
                        .await
                })
                .unwrap_or(false)
            });
            present.store(available, Ordering::Relaxed);
            std::thread::sleep(Duration::from_secs(5));
        }
    });
}

#[cfg(not(target_os = "linux"))]
fn probe(_: Arc<AtomicBool>, _: Arc<AtomicBool>) {}
