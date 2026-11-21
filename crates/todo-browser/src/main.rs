#![cfg_attr(target_os = "emscripten", no_main)]

mod app;
mod exports;
mod host;

use ferroui_base::media::FontManagerOptions;
use ferroui_browser::{BrowserAccessibilityMode, BrowserAppBuilder, BrowserPlatformOptions};
use ferroui_controls::AppBuilder;
use ferroui_fonts_inter::AppBuilderExtension;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = runMain)]
pub fn run_main(query: &str) {
    app::set_demo(
        query
            .split('&')
            .any(|part| part.trim_start_matches('?') == "demo=1"),
    );
    AppBuilder::configure::<app::App>()
        .with_inter_font()
        .with(Rc::new(FontManagerOptions {
            default_family_name: Some("fonts:Inter#Inter".into()),
            ..Default::default()
        }))
        .start_browser_app(
            "out",
            Some(BrowserPlatformOptions {
                accessibility: BrowserAccessibilityMode::Enabled,
                ..Default::default()
            }),
        );
}

#[cfg(not(target_os = "emscripten"))]
fn main() {
    println!("Build the browser application with scripts/build-browser.sh");
}
