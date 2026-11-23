mod account_workspace;
mod back_stack;
mod bulk;
mod commands;
pub mod gestures;
mod history;
mod history_commands;
mod integration;
mod layout;
mod list_commands;
mod naming;
mod navigation;
mod persistence;
mod presentation;
mod refresh;
mod repeat_draft;
mod scheduling;
mod selection;
mod services;
mod task_clipboard;

pub use integration::OutlookCategory;
pub use layout::{PaneLayout, COMPACT_WIDTH, DETAIL_WIDTH, MINIMUM_LIST_WIDTH, NAVIGATION_WIDTH};
pub use naming::{RenameSession, RenameTarget};
pub use navigation::{view_title, Panel};
pub use repeat_draft::{RepeatDraft, RepeatValidation};
pub use scheduling::{parse_reminder_time, reminder_in_one_hour};
pub use services::{HostRequest, HostServices, SaveCallback};
pub use task_clipboard::{
    clipboard_task_lines, TaskClipboard, TaskClipboardOffer, TaskPasteTarget,
    MAX_TASK_CLIPBOARD_BYTES,
};

use ferroui_dynamic_data::SourceCache;
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::{reactive_object, IReactiveObjectExt, ReactiveCommand, ReactiveObject};
use std::cell::{Cell, Ref, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;
use todo_domain::{Action, AppState, Date, Id, SortOrder, Task, View};

/// Application lifetime MVVM state. Views never mutate the document directly.
pub struct AppViewModel {
    reactive: ReactiveObject,
    revision: RefCell<u64>,
    document: RefCell<AppState>,
    today: Cell<Date>,
    current_minutes: Cell<u16>,
    view: RefCell<View>,
    navigation_history: RefCell<Vec<View>>,
    refreshing: Cell<bool>,
    selected: Cell<Option<Id>>,
    selection: RefCell<BTreeSet<Id>>,
    selection_mode: Cell<bool>,
    selection_anchor: Cell<Option<Id>>,
    task_cursor: Cell<Option<Id>>,
    panel: RefCell<Panel>,
    rename: RefCell<Option<RenameSession>>,
    sidebar: Cell<bool>,
    navigation_open: Cell<bool>,
    step_drafts: RefCell<BTreeMap<Id, String>>,
    viewport: Cell<f64>,
    system_dark: Cell<bool>,
    cloud_connected: Cell<bool>,
    cloud_supported: Cell<bool>,
    cloud_configured: Cell<bool>,
    cloud_work_account: Cell<bool>,
    cloud_label: RefCell<String>,
    cloud_hint: RefCell<String>,
    background_supported: Cell<bool>,
    background_reminders_supported: Cell<bool>,
    background_reminders_status: RefCell<String>,
    outlook_categories: RefCell<Vec<OutlookCategory>>,
    completed_expanded: Cell<bool>,
    sort: Cell<SortOrder>,
    sort_reversed: Cell<bool>,
    status: RefCell<String>,
    status_undo: Cell<bool>,
    undo: RefCell<VecDeque<history::HistoryEntry>>,
    redo: RefCell<Vec<history::HistoryEntry>>,
    applying: Cell<bool>,
    pending: RefCell<VecDeque<commands::PendingAction>>,
    delivered_reminders: RefCell<BTreeSet<(Id, Date, u16)>>,
    dirty: Cell<bool>,
    save: SaveCallback,
    services: RefCell<HostServices>,
    task_clipboard: RefCell<Option<task_clipboard::ClipboardSnapshot>>,
    clipboard_epoch: Cell<u64>,
    pub tasks: SourceCache<Task, Id>,
    pub execute: ReactiveCommand<Option<Action>, ()>,
}

reactive_object!(AppViewModel);

impl AppViewModel {
    pub fn new(state: AppState, today: Date, save: SaveCallback) -> Rc<Self> {
        let tasks = SourceCache::new(|task: &Task| task.id);
        tasks.edit(|cache| cache.add_or_update_items(&mut state.tasks.clone().into_iter()));
        Rc::new_cyclic(|weak: &std::rc::Weak<Self>| {
            let receiver = weak.clone();
            Self {
                reactive: ReactiveObject::new(),
                revision: RefCell::new(0),
                document: RefCell::new(state),
                today: Cell::new(today),
                current_minutes: Cell::new(0),
                view: RefCell::new(View::MyDay),
                navigation_history: RefCell::new(Vec::new()),
                refreshing: Cell::new(false),
                selected: Cell::new(None),
                selection: RefCell::new(BTreeSet::new()),
                selection_mode: Cell::new(false),
                selection_anchor: Cell::new(None),
                task_cursor: Cell::new(None),
                panel: RefCell::new(Panel::None),
                rename: RefCell::new(None),
                sidebar: Cell::new(true),
                navigation_open: Cell::new(false),
                step_drafts: RefCell::new(BTreeMap::new()),
                viewport: Cell::new(1440.0),
                system_dark: Cell::new(false),
                cloud_connected: Cell::new(false),
                cloud_supported: Cell::new(false),
                cloud_configured: Cell::new(false),
                cloud_work_account: Cell::new(false),
                cloud_label: RefCell::new(String::new()),
                cloud_hint: RefCell::new(String::new()),
                background_supported: Cell::new(false),
                background_reminders_supported: Cell::new(false),
                background_reminders_status: RefCell::new(String::new()),
                outlook_categories: RefCell::new(Vec::new()),
                completed_expanded: Cell::new(true),
                sort: Cell::new(SortOrder::Manual),
                sort_reversed: Cell::new(false),
                status: RefCell::new(String::new()),
                status_undo: Cell::new(false),
                undo: RefCell::new(VecDeque::new()),
                redo: RefCell::new(Vec::new()),
                applying: Cell::new(false),
                pending: RefCell::new(VecDeque::new()),
                delivered_reminders: RefCell::new(BTreeSet::new()),
                dirty: Cell::new(false),
                save,
                services: RefCell::new(HostServices::default()),
                task_clipboard: RefCell::new(None),
                clipboard_epoch: Cell::new(0),
                tasks,
                execute: ReactiveCommand::create_param_with(
                    move |action: Option<Action>| {
                        if let Some(vm) = receiver.upgrade() {
                            vm.apply(action);
                        }
                    },
                    None,
                    Some(ImmediateScheduler::instance()),
                ),
            }
        })
    }

    pub fn dispatch(&self, action: Action) {
        if self.applying.get() {
            self.pending
                .borrow_mut()
                .push_back(commands::PendingAction::new(action));
            return;
        }
        let _subscription = self.execute.execute(Some(action)).subscribe_next(|_| {});
    }

    pub fn state(&self) -> Ref<'_, AppState> {
        self.document.borrow()
    }
    pub fn revision(&self) -> u64 {
        *self.revision.borrow()
    }
    pub fn today(&self) -> Date {
        self.today.get()
    }
    pub fn current_minutes(&self) -> u16 {
        self.current_minutes.get()
    }
    pub fn set_current_minutes(&self, minutes: u16) {
        self.current_minutes.set(minutes.min(1439));
    }
    pub fn view(&self) -> View {
        self.view.borrow().clone()
    }
    pub fn selected(&self) -> Option<Id> {
        self.selected.get()
    }
    pub fn panel(&self) -> Panel {
        self.panel.borrow().clone()
    }
    pub fn status(&self) -> String {
        self.status.borrow().clone()
    }
    pub fn completed_expanded(&self) -> bool {
        self.completed_expanded.get()
    }
    pub fn viewport(&self) -> f64 {
        self.viewport.get()
    }
    pub fn is_dark(&self) -> bool {
        match self.state().settings.theme {
            todo_domain::Theme::Dark => true,
            todo_domain::Theme::Light => false,
            todo_domain::Theme::System => self.system_dark.get(),
        }
    }
    pub fn set_system_dark(&self, dark: bool) {
        if self.system_dark.replace(dark) != dark {
            self.notify();
        }
    }
    pub fn cloud_connected(&self) -> bool {
        self.cloud_connected.get()
    }
    pub fn cloud_label(&self) -> String {
        self.cloud_label.borrow().clone()
    }
    pub fn set_cloud_state(&self, connected: bool, label: &str) {
        let changed = self.cloud_connected.replace(connected) != connected
            || *self.cloud_label.borrow() != label;
        *self.cloud_label.borrow_mut() = label.to_owned();
        if changed {
            self.notify();
        }
    }
    pub fn selection(&self) -> BTreeSet<Id> {
        self.selection.borrow().clone()
    }

    pub fn set_services(&self, services: HostServices) {
        *self.services.borrow_mut() = services;
    }
    pub fn request(&self, request: HostRequest) {
        let callback = self.services.borrow().request.clone();
        callback(request);
    }

    pub fn navigate(&self, view: View) {
        self.remember_navigation(&view);
        self.navigate_core(view);
    }

    pub(crate) fn navigate_core(&self, view: View) {
        self.rename.borrow_mut().take();
        if let View::List(id) = &view {
            if let Some(list) = self.document.borrow().list(*id) {
                self.sort.set(list.sort);
                self.sort_reversed.set(list.sort_reversed);
            }
        } else {
            self.sort_reversed.set(false);
            self.sort.set(if view == View::Planned {
                SortOrder::DueDate
            } else {
                SortOrder::Manual
            });
        }
        *self.view.borrow_mut() = view;
        self.selected.set(None);
        self.selection.borrow_mut().clear();
        self.selection_mode.set(false);
        self.selection_anchor.set(None);
        self.task_cursor.set(None);
        *self.panel.borrow_mut() = Panel::None;
        self.navigation_open.set(false);
        self.notify();
    }

    pub fn select(&self, id: Option<Id>) {
        self.rename.borrow_mut().take();
        self.navigation_open.set(false);
        self.selected.set(id);
        if let Some(id) = id {
            self.task_cursor.set(Some(id));
            self.selection_mode.set(false);
            self.selection.borrow_mut().clear();
            self.selection_anchor.set(None);
        }
        *self.panel.borrow_mut() = Panel::None;
        self.notify();
    }

    pub fn toggle_selection(&self, id: Id) {
        let mut selection = self.selection.borrow_mut();
        if !selection.remove(&id) {
            selection.insert(id);
        }
        drop(selection);
        self.notify();
    }

    pub fn show_panel(&self, panel: Panel) {
        if matches!(panel, Panel::Settings | Panel::Suggestions) {
            self.navigation_open.set(false);
        }
        *self.panel.borrow_mut() = panel;
        self.notify();
    }
    pub fn toggle_completed(&self) {
        self.completed_expanded.set(!self.completed_expanded.get());
        self.notify();
    }
    pub fn set_today(&self, today: Date) {
        if self.today.replace(today) != today {
            self.notify();
        }
    }
    pub fn report_error(&self, error: String) {
        self.set_status(error);
    }
    pub fn clear_status(&self) {
        self.set_status(String::new());
    }
    pub fn set_status(&self, status: String) {
        self.update_status(status, false);
    }
    pub(crate) fn set_undo_status(&self, status: String) {
        self.update_status(status, self.can_undo());
    }
    pub fn status_can_undo(&self) -> bool {
        self.status_undo.get() && self.can_undo()
    }
    fn update_status(&self, status: String, undo: bool) {
        *self.status.borrow_mut() = status;
        self.status_undo.set(undo);
        // History/import callers also use this as their completed-operation notification.
        // Repeated "Change undone" messages must still reconcile the changed document.
        self.notify();
    }
    pub(crate) fn notify(&self) {
        let next = self.revision().wrapping_add(1);
        self.raise_and_set_if_changed(&self.revision, next, "Revision");
    }
}
