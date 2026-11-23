#![cfg_attr(target_os = "emscripten", no_main)]

mod app;
mod exports;
mod host;

use ferroui_browser::{BrowserAccessibilityMode, BrowserAppBuilder, BrowserPlatformOptions};
use ferroui_controls::AppBuilder;
use ferroui_fonts_inter::AppBuilderExtension;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = runMain)]
pub fn run_main(query: &str, emoji_font: Vec<u8>, cjk_font: Vec<u8>) -> Result<(), JsValue> {
    app::set_demo(
        query
            .split('&')
            .any(|part| part.trim_start_matches('?') == "demo=1"),
    );
    let font_error = Rc::new(RefCell::new(None));
    let font_result = font_error.clone();
    let font_bytes = RefCell::new(Some((emoji_font, cjk_font)));
    AppBuilder::configure::<app::App>()
        .with_inter_font()
        .with(Rc::new(todo_controls::font_options()))
        .configure_fonts(move |manager| {
            if let Some((emoji, cjk)) = font_bytes.borrow_mut().take() {
                *font_result.borrow_mut() = todo_controls::install_emoji_font(manager, &emoji)
                    .and_then(|()| todo_controls::install_cjk_font(manager, &cjk))
                    .err();
            }
        })
        .start_browser_app(
            "out",
            Some(BrowserPlatformOptions {
                accessibility: BrowserAccessibilityMode::Enabled,
                // Images are copied into the document; a transient file input
                // works consistently across browsers without retaining grants.
                prefer_file_dialog_polyfill: true,
                ..Default::default()
            }),
        );
    let error = font_error.borrow_mut().take();
    match error {
        Some(error) => Err(JsValue::from_str(&error)),
        None => Ok(()),
    }
}

#[cfg(not(target_os = "emscripten"))]
fn main() {
    println!("Build the browser application with scripts/build-browser.sh");
}
