mod persistence;
mod services;

use chrono::{Local, Timelike};
use ferroui_base::media::FontManagerOptions;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_controls::{AppBuilder, Application, Control, Window};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_fonts_inter::AppBuilderExtension;
use std::rc::Rc;
use std::time::Duration;
use todo_domain::AppState;
use todo_state::AppViewModel;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let demo = args.iter().any(|arg| arg == "--demo");
    AppBuilder::configure::<Application>()
        .use_platform_detect()
        .with_inter_font()
        .with(Rc::new(FontManagerOptions {
            default_family_name: Some("fonts:Inter#Inter".into()),
            ..Default::default()
        }))
        .start(
            move |app, _| {
                todo_controls::install_theme(app);
                let today = Local::now().date_naive();
                let directory = persistence::data_directory();
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
                let window = Window::new();
                window.set_title(Some("To Do".into()));
                window.set_width(1240.0);
                window.set_height(820.0);
                window.set_min_width(380.0);
                window.set_min_height(480.0);
                services::install(&vm, &window, directory);
                if let Some(error) = error {
                    vm.report_error(error);
                }
                window.set_content(Some(Control::boxed(&todo_ui::create_view(vm.clone()))));
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
                window.closed(move || timer.dispose()).detach();
                app.run_window(window);
            },
            &args,
        );
}
