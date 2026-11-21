use wasm_bindgen::prelude::*;

#[wasm_bindgen(raw_module = "./host.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    pub fn load() -> Result<String, JsValue>;
    #[wasm_bindgen(catch)]
    pub fn save(contents: &str) -> Result<(), JsValue>;
    pub fn today() -> String;
    pub fn width() -> f64;
    #[wasm_bindgen(js_name = systemDark)]
    pub fn system_dark() -> bool;
    pub fn request(json: &str);
    #[wasm_bindgen(js_name = preserveRecovery)]
    pub fn preserve_recovery(contents: &str);
}

pub fn error_message(error: JsValue) -> String {
    error.as_string().unwrap_or_else(|| {
        "Storage is full or blocked by this browser. Export your tasks to keep a backup.".into()
    })
}
