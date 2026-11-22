//! Small worker-only module: no UI platform, browser lifecycle, or account state.
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = importPstData)]
pub fn import_pst_data(bytes: Vec<u8>, current_json: &str, today: &str) -> Result<String, JsValue> {
    let current = todo_domain::AppState::from_json(current_json)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let today = todo_domain::Date::parse_from_str(today, "%Y-%m-%d")
        .map_err(|_| JsValue::from_str("Invalid import date."))?;
    let result = todo_interop::import_pst_owned(bytes, &current, today)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    serde_json::to_string(&result).map_err(|error| JsValue::from_str(&error.to_string()))
}
