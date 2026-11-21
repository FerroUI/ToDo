use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{Application, ApplicationImpl, ApplicationImplExt, NewApplication};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use todo_domain::{AppState, Date};
use todo_state::{AppViewModel, HostServices};

ferroui_base::ferro_thread_local! {
    static VIEW_MODEL: RefCell<Option<Rc<AppViewModel>>> = const { RefCell::new(None) };
    static DEMO: Cell<bool> = const { Cell::new(false) };
}

pub fn set_demo(value: bool) {
    DEMO.with(|demo| demo.set(value));
}

pub fn view_model() -> Option<Rc<AppViewModel>> {
    VIEW_MODEL.with(|vm| vm.borrow().clone())
}

#[repr(C)]
pub struct App {
    base: Application,
}

ferro_class!(App: Application);
ferro_impl_classes!(App: FerroObjectImpl);

impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}

impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        todo_controls::install_theme(this);
    }

    fn on_framework_initialization_completed(this: &Self) {
        let today = Date::parse_from_str(&crate::host::today(), "%Y-%m-%d")
            .expect("The browser supplies a valid local civil date");
        let (state, load_error) = read_state(today);
        let vm = AppViewModel::new(
            state,
            today,
            Rc::new(|json| crate::host::save(json).map_err(crate::host::error_message)),
        );
        vm.set_viewport(crate::host::width());
        vm.set_system_dark(crate::host::system_dark());
        vm.set_services(HostServices {
            request: Rc::new(|request| match serde_json::to_string(&request) {
                Ok(json) => crate::host::request(&json),
                Err(error) => {
                    if let Some(vm) = view_model() {
                        vm.report_error(error.to_string());
                    }
                }
            }),
        });
        VIEW_MODEL.with(|slot| *slot.borrow_mut() = Some(vm.clone()));
        if let Some(error) = load_error {
            vm.report_error(error);
        }
        if let Some(lifetime) = this
            .application_lifetime()
            .as_ref()
            .and_then(|lifetime| lifetime.as_single_view_application_lifetime())
        {
            lifetime.set_main_view(Some(todo_ui::create_view(vm)));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}

fn read_state(today: Date) -> (AppState, Option<String>) {
    if DEMO.with(|demo| demo.get()) {
        return (AppState::sample(today), None);
    }
    match crate::host::load() {
        Ok(json) if json.trim().is_empty() => (AppState::new(), None),
        Ok(json) => match AppState::from_json(&json) {
            Ok(state) => (state, None),
            Err(error) => {
                crate::host::preserve_recovery(&json);
                (AppState::new(), Some(format!("Your saved tasks could not be read. A recovery copy was preserved. {error}")))
            }
        },
        Err(error) => (
            AppState::new(),
            Some(format!(
                "Browser storage is unavailable: {}",
                crate::host::error_message(error)
            )),
        ),
    }
}
