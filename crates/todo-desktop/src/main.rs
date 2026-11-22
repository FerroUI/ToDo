mod native;
mod persistence;
mod services;

#[global_allocator]
static ALLOCATOR: todo_interop::allocation_budget::ParserAllocator =
    todo_interop::allocation_budget::ParserAllocator::new();

use chrono::{Local, Timelike};
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::{AppBuilder, Application, Control, TopLevel, Window};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_fonts_inter::AppBuilderExtension;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;
use todo_domain::AppState;
use todo_state::AppViewModel;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "--prepare-pst") {
        std::process::exit(native::pst_child());
    }
    if args.first().is_some_and(|arg| arg == "--credential-vault") {
        std::process::exit(native::credential_child());
    }
    let demo = args.iter().any(|arg| arg == "--demo");
    let directory = persistence::data_directory();
    let _instance = if demo {
        None
    } else {
        match persistence::lock_directory(&directory) {
            Ok(lock) => Some(lock),
            Err(error) => {
                eprintln!("{error}");
                return;
            }
        }
    };
    AppBuilder::configure::<Application>()
        .use_platform_detect()
        .with_inter_font()
        .with(Rc::new(todo_controls::font_options()))
        .configure_fonts(|manager| {
            todo_controls::install_emoji_font(
                manager,
                include_bytes!("../../todo-controls/assets/fonts/Noto-COLRv1.ttf"),
            )
            .expect("the bundled Noto emoji font loads");
        })
        .start(
            move |app, _| {
                todo_controls::install_theme(app);
                let today = Local::now().date_naive();
                let directory = directory.clone();
                let (state, error) = if demo {
                    (AppState::sample(today), None)
                } else {
                    persistence::load(&directory)
                };
                let storage = directory.clone();
                let vm = AppViewModel::new(
                    state,
                    today,
                    Rc::new(move |json| {
                        if demo {
                            Ok(())
                        } else {
                            persistence::save(&storage, json)
                        }
                    }),
                );
                let now = Local::now();
                vm.set_current_minutes((now.hour() * 60 + now.minute()) as u16);
                let window = Window::new();
                window.set_title(Some("To Do".into()));
                window.set_width(1240.0);
                window.set_height(820.0);
                window.set_min_width(380.0);
                window.set_min_height(480.0);
                services::install(&vm, &window, directory, demo);
                if let Some(error) = error {
                    vm.report_error(error);
                }
                window.set_content(Some(Control::boxed(&todo_ui::create_view(vm.clone()))));
                let back_window = window.downgrade();
                let back = window.back_requested(move |_, event| {
                    if back_window
                        .upgrade()
                        .is_some_and(|window| todo_ui::navigate_back(&window))
                    {
                        event.set_handled(true);
                    }
                });
                let layout_vm = Rc::downgrade(&vm);
                let layout_window = window.downgrade();
                window
                    .layout_updated(move || {
                        if let (Some(vm), Some(window)) =
                            (layout_vm.upgrade(), layout_window.upgrade())
                        {
                            vm.set_viewport(window.client_size().width);
                        }
                    })
                    .detach();
                let closing_vm = Rc::downgrade(&vm);
                let closing_window = window.downgrade();
                window
                    .closing(move |_| {
                        if let (Some(vm), Some(window)) =
                            (closing_vm.upgrade(), closing_window.upgrade())
                        {
                            todo_ui::commit_pending_edits(&vm, &window);
                        }
                    })
                    .detach();
                let weak = Rc::downgrade(&vm);
                let timer = DispatcherTimer::run(
                    move || {
                        let Some(vm) = weak.upgrade() else {
                            return false;
                        };
                        let now = Local::now();
                        vm.tick(now.date_naive(), (now.hour() * 60 + now.minute()) as u16);
                        true
                    },
                    Duration::from_secs(30),
                    DispatcherPriority::BACKGROUND,
                );
                let timer = RefCell::new(Some(timer));
                let closed_window = window.downgrade();
                window
                    .closed(move || {
                        if let Some(window) = closed_window.upgrade() {
                            window.remove_handler(TopLevel::back_requested_event(), back);
                        }
                        let stopped = timer.borrow_mut().take();
                        if let Some(timer) = stopped {
                            timer.dispose();
                        }
                    })
                    .detach();
                app.run_window(window);
            },
            &args,
        );
}
