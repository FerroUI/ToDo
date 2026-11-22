use ferroui_base::input::InputElement;
use ferroui_base::{Point, Visual};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Application, Button, Calendar, ComboBox, ListBox, TextBlock, TextBox, TopLevel,
};
use serde_json::json;
use todo_domain::{Action, Attachment, Date};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = exportData)]
pub fn export_data() -> Result<String, JsValue> {
    flush_edits();
    crate::app::view_model()
        .ok_or_else(|| JsValue::from_str("The app is starting"))?
        .export_json()
        .map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen(js_name = flushEdits)]
pub fn flush_edits() {
    if let (Some(vm), Some(root)) = (crate::app::view_model(), top_level()) {
        todo_ui::commit_pending_edits(&vm, &root);
    }
}

#[wasm_bindgen(js_name = navigateBack)]
pub fn navigate_back() -> bool {
    top_level().is_some_and(|root| todo_ui::navigate_back(&root))
}

#[wasm_bindgen(js_name = finishRefresh)]
pub fn finish_refresh() {
    if let Some(vm) = crate::app::view_model() {
        vm.finish_refresh();
    }
}

#[wasm_bindgen(js_name = importData)]
pub fn import_data(json: &str) -> Result<(), JsValue> {
    crate::app::view_model()
        .ok_or_else(|| JsValue::from_str("The app is starting"))?
        .import_json(json)
        .map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen(js_name = replaceFromCloud)]
pub fn replace_from_cloud(json: &str, expected: &str) -> Result<(), JsValue> {
    let vm = crate::app::view_model().ok_or_else(|| JsValue::from_str("The app is starting"))?;
    if vm
        .export_json()
        .map_err(|error| JsValue::from_str(&error))?
        != expected
    {
        return Err(JsValue::from_str(
            "Tasks changed locally while syncing. Sync again; your edits were kept.",
        ));
    }
    vm.replace_from_cloud(json)
        .map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen(js_name = prepareCloudData)]
pub fn prepare_cloud_data(json: &str) -> Result<String, JsValue> {
    crate::app::view_model()
        .ok_or_else(|| JsValue::from_str("The app is starting"))?
        .prepare_cloud_json(json)
        .map_err(|error| JsValue::from_str(&error))
}

#[wasm_bindgen(js_name = cloudState)]
pub fn cloud_state(connected: bool, label: &str) {
    if let Some(vm) = crate::app::view_model() {
        vm.set_cloud_state(connected, label);
    }
}

#[wasm_bindgen(js_name = hostError)]
pub fn host_error(message: &str) {
    if let Some(vm) = crate::app::view_model() {
        vm.report_error(message.to_owned());
    }
}

#[wasm_bindgen(js_name = attachFile)]
pub fn attach_file(
    task_id: &str,
    name: &str,
    mime_type: &str,
    size: f64,
    key: &str,
) -> Result<(), JsValue> {
    let vm = crate::app::view_model().ok_or_else(|| JsValue::from_str("The app is starting"))?;
    let task_id = task_id
        .parse()
        .map_err(|_| JsValue::from_str("Invalid task identifier"))?;
    let limit = 25 * 1024 * 1024;
    if !size.is_finite() || size < 0.0 || size.fract() != 0.0 || size > limit as f64 {
        return Err(JsValue::from_str("This attachment has an invalid size"));
    }
    let total = vm
        .state()
        .task(task_id)
        .ok_or_else(|| JsValue::from_str("This task no longer exists"))?
        .attachments
        .iter()
        .try_fold(0_u64, |total, file| total.checked_add(file.size));
    if total
        .and_then(|total| total.checked_add(size as u64))
        .is_none_or(|total| total > limit)
    {
        return Err(JsValue::from_str(
            "Attachments for a task must total 25 MB or less",
        ));
    }
    vm.dispatch(Action::AddAttachment {
        task_id,
        attachment: Attachment {
            id: 0,
            name: name.into(),
            mime_type: mime_type.into(),
            size: size as u64,
            storage_key: key.into(),
        },
    });
    let attached = vm
        .state()
        .task(task_id)
        .is_some_and(|task| task.attachments.iter().any(|file| file.storage_key == key));
    if attached {
        Ok(())
    } else {
        Err(JsValue::from_str(&vm.status()))
    }
}

#[wasm_bindgen]
pub fn tick(date: &str, minutes: u32) {
    if let (Some(vm), Ok(date)) = (
        crate::app::view_model(),
        Date::parse_from_str(date, "%Y-%m-%d"),
    ) {
        vm.tick(date, minutes as u16);
    }
}

#[wasm_bindgen(js_name = resizeWindow)]
pub fn resize_window(width: f64) {
    if width.is_finite() && width > 0.0 {
        if let Some(vm) = crate::app::view_model() {
            // TopLevel already pads ComponentHost for device cutouts. Resolve pane
            // budgets against that usable width, especially in phone landscape.
            let insets = top_level()
                .and_then(|root| root.insets_manager())
                .map(|manager| manager.safe_area_padding())
                .unwrap_or_default();
            vm.set_viewport((width - insets.left - insets.right).max(0.0));
        }
    }
}

#[wasm_bindgen(js_name = updateTheme)]
pub fn update_theme(dark: bool) {
    if let Some(vm) = crate::app::view_model() {
        vm.set_system_dark(dark);
    }
}

fn top_level() -> Option<ferroui_base::Ref<TopLevel>> {
    Application::current()?
        .application_lifetime()?
        .as_single_top_level_application_lifetime()?
        .top_level()
}

/// Local automation can inspect rendered controls without depending on canvas pixels.
#[wasm_bindgen(js_name = inspectView)]
pub fn inspect_view() -> String {
    let Some(top_level) = top_level() else {
        return "null".into();
    };
    let viewport = top_level.bounds();
    let elements = top_level.get_visual_descendants().filter_map(|visual| {
        let name = visual.name();
        let text = if let Some(text) = visual.cast::<TextBlock>() { text.text() }
            else if let Some(text) = visual.cast::<TextBox>() { text.text() }
            else if visual.cast::<Button>().is_some() || visual.cast::<ferroui_controls::MenuItem>().is_some()
                || visual.cast::<Calendar>().is_some() || visual.cast::<ComboBox>().is_some()
                || visual.cast::<ListBox>().is_some() || name.is_some() { None }
            else { return None; };
        let bounds = visual.bounds();
        if !visual.is_effectively_visible() || bounds.width <= 0.0 || bounds.height <= 0.0 { return None; }
        let origin = visual.translate_point(Point::new(0.0, 0.0), &top_level)?;
        if origin.x >= viewport.width || origin.y >= viewport.height || origin.x + bounds.width <= 0.0 || origin.y + bounds.height <= 0.0 { return None; }
        let center = Point::new(origin.x + bounds.width / 2.0, origin.y + bounds.height / 2.0);
        let hit = top_level.input_hit_test_with(center, false).is_some_and(|hit| {
            let hit: &Visual = &hit;
            std::ptr::eq(hit, &*visual) || visual.is_visual_ancestor_of(hit) || hit.is_visual_ancestor_of(&visual)
        });
        let text_box_name = visual.cast::<TextBox>().and_then(|field| field.name()).or_else(|| {
            visual.get_visual_ancestors().filter_map(|parent| parent.cast::<TextBox>())
                .find_map(|field| field.name())
        });
        Some(json!({ "type": visual.get_type().name(), "name": name, "text": text,
            "textBoxName": text_box_name,
            "automationName": AutomationProperties::get_name(&visual),
            "automationItemStatus": AutomationProperties::get_item_status(&visual),
            "focused": visual.cast::<InputElement>().is_some_and(|input| input.is_focused()),
            "x": origin.x, "y": origin.y, "width": bounds.width, "height": bounds.height, "hit": hit }))
    }).collect::<Vec<_>>();
    json!({ "width": viewport.width, "height": viewport.height, "elements": elements }).to_string()
}
