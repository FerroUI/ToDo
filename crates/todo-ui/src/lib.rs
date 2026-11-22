mod detail;
mod dialogs;
mod drag;
mod editing;
mod inline_name;
mod list;
mod menus;
mod rows;
mod settings;
mod shell;
mod sidebar;
mod suggestions;
mod touch;

use ferroui_base::Ref;
use ferroui_controls::{Application, Control, InputPaneAwareBehavior};
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_state::AppViewModel;

pub use editing::commit_pending_edits;
pub use shell::{can_navigate_back, navigate_back};

/// Retains component owners, subscriptions and recyclable item templates for the view lifetime.
pub fn create_view(vm: Rc<AppViewModel>) -> Ref<Control> {
    ComponentHost::with_view(move || {
        let revision = signal(vm.revision());
        let rows = Rc::new(rows::Rows::new(vm.clone()));
        rows.synchronize(&vm);
        let update_vm = vm.clone();
        let update_rows = rows.clone();
        let subscription = vm.changed().subscribe_next(move |_| {
            update_rows.synchronize(&update_vm);
            revision.set(update_vm.revision());
        });
        on_cleanup(move || subscription.dispose());
        let view_model = vm.clone();
        // The outer host owns safe-area padding; this retained decorator handles
        // only the keyboard's intersection with the remaining content area.
        input_pane_aware_decorator()
            .behavior(InputPaneAwareBehavior::Resize)
            .child(rerender(move || {
                revision.get();
                if let Some(app) = Application::current() {
                    todo_controls::set_dark_mode(&app, view_model.is_dark());
                }
                shell::view(view_model.clone(), rows.clone())
            }))
    })
    .upcast()
}

pub(crate) fn palette(vm: &AppViewModel) -> todo_controls::Palette {
    todo_controls::Palette::for_dark(vm.is_dark())
}
