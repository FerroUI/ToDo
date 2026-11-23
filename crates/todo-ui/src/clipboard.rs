//! Task commands share the platform clipboard with native text editors.
use ferroui_base::input::platform::IClipboard;
use ferroui_base::input::{DataTransfer, IAsyncDataTransfer};
use ferroui_base::threading::Dispatcher;
use ferroui_controls::{Control, TopLevel};
use ferroui_view::Element;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::{Id, View};
use todo_state::AppViewModel;
fn platform(anchor: &Control) -> Option<Rc<dyn IClipboard>> {
    TopLevel::get_top_level(Some(anchor)).and_then(|root| root.clipboard())
}

pub(crate) fn copy(anchor: &Control, vm: Rc<AppViewModel>, ids: Vec<Id>, cut: bool) {
    write(platform(anchor), vm, ids, cut);
}

fn write(clipboard: Option<Rc<dyn IClipboard>>, vm: Rc<AppViewModel>, ids: Vec<Id>, cut: bool) {
    let Some(clipboard) = clipboard else {
        vm.report_error("System clipboard is unavailable".into());
        return;
    };
    let prepared = (|| {
        let offer = vm.prepare_task_clipboard(&ids, cut)?;
        let item = crate::task_transfer::item(&offer.data)?;
        let data = DataTransfer::new();
        data.add(item);
        Ok::<_, String>((offer, data))
    })();
    let (offer, data) = match prepared {
        Ok(value) => value,
        Err(error) => {
            vm.report_error(error);
            return;
        }
    };
    let write = clipboard.set_data_async(Some(data));
    Dispatcher::ui_thread().spawn_local(async move {
        match write.await {
            Ok(()) => {
                if let Err(error) = vm.accept_task_clipboard(offer) {
                    vm.report_error(error);
                }
            }
            Err(error) => vm.report_error(format!("Could not write to the clipboard: {error}")),
        }
    });
}

pub(crate) fn paste(anchor: &Control, vm: Rc<AppViewModel>, view: View) {
    read(platform(anchor), vm, view);
}

fn read(clipboard: Option<Rc<dyn IClipboard>>, vm: Rc<AppViewModel>, view: View) {
    let Some(clipboard) = clipboard else {
        vm.report_error("System clipboard is unavailable".into());
        return;
    };
    let target = vm.task_paste_target(view);
    let pending = clipboard.try_get_data_async();
    Dispatcher::ui_thread().spawn_local(async move {
        let result = async {
            let data = pending
                .await
                .map_err(|error| format!("Could not read the clipboard: {error}"))?
                .ok_or("The clipboard is empty")?;
            let data = Using(data);
            let (payload, text) = crate::task_transfer::read_clipboard(&*data.0).await?;
            vm.paste_task_clipboard(target, payload, &text)
        }
        .await;
        if let Err(error) = result {
            vm.report_error(error);
        }
    });
}

struct Using(Rc<dyn IAsyncDataTransfer>);
impl Drop for Using {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

pub(crate) fn task_commands(
    anchor: &Control,
    vm: Rc<AppViewModel>,
    ids: Rc<[Id]>,
    can_cut: bool,
) -> Vec<Element> {
    let clipboard = platform(anchor);
    let copy_clipboard = clipboard.clone();
    let copy_vm = vm.clone();
    let copy_ids = ids.clone();
    let mut items = vec![crate::menus::command_detail(
        "Copy tasks",
        "Ctrl+C",
        Icon::Copy,
        move || {
            write(
                copy_clipboard.clone(),
                copy_vm.clone(),
                copy_ids.to_vec(),
                false,
            );
        },
    )];
    if can_cut {
        items.push(crate::menus::command_detail(
            "Cut tasks",
            "Ctrl+X",
            Icon::MoveToList,
            move || {
                write(clipboard.clone(), vm.clone(), ids.to_vec(), true);
            },
        ));
    }
    items
}

pub(crate) fn paste_command(anchor: &Control, vm: Rc<AppViewModel>, view: View) -> Element {
    let clipboard = platform(anchor);
    crate::menus::command_detail("Paste tasks", "Ctrl+V", Icon::Copy, move || {
        read(clipboard.clone(), vm.clone(), view.clone())
    })
}
