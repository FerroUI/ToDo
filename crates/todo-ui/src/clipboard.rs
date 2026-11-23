//! Task commands share the platform clipboard with native text editors.
use ferroui_base::input::platform::IClipboard;
use ferroui_base::input::{
    AsyncDataTransferExtensions, DataFormat, DataFormatOf, DataTransfer, DataTransferItem,
    IAsyncDataTransfer,
};
use ferroui_base::threading::Dispatcher;
use ferroui_controls::{Control, TopLevel};
use ferroui_view::Element;
use std::rc::Rc;
use todo_controls::Icon;
use todo_domain::{Id, View};
use todo_state::{AppViewModel, TaskClipboard};

fn format() -> DataFormatOf<Rc<[u8]>> {
    DataFormat::create_bytes_application_format("FerroUI.ToDo.Tasks")
}
fn html_format() -> DataFormatOf<String> {
    DataFormat::create_string_platform_format("text/html")
}
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
        let item = DataTransferItem::new();
        item.set(&DataFormat::text(), Some(offer.data.text()));
        item.set(&html_format(), Some(offer.data.to_html()?));
        item.set(
            &format(),
            Some(Rc::<[u8]>::from(offer.data.to_json()?.into_bytes())),
        );
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
            let bytes = data
                .0
                .try_get_value_async(&format())
                .await
                .map_err(|error| error.to_string())?;
            let payload = if let Some(bytes) = bytes {
                Some(TaskClipboard::from_json(
                    std::str::from_utf8(&bytes)
                        .map_err(|_| "The clipboard task data is invalid")?,
                )?)
            } else {
                let html = data
                    .0
                    .try_get_value_async(&html_format())
                    .await
                    .map_err(|error| error.to_string())?;
                match html {
                    Some(html) => TaskClipboard::from_html(&html)?,
                    None => None,
                }
            };
            let text = data
                .0
                .try_get_value_async(&DataFormat::text())
                .await
                .map_err(|error| error.to_string())?
                .unwrap_or_default();
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
