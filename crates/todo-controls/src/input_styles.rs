//! Opt-in adjustments for native editors embedded in compact app controls.
use ferroui_base::Visual;
use ferroui_controls::Application;
use ferroui_view::style;

pub(crate) fn install(app: &Application) {
    // WinUI reveals DeleteButton through a focus-triggered theme style. Match that
    // trigger priority; app styles apply after control/template-parent themes.
    // Callers keep their own clear action or dedicate the small field to its value.
    app.styles().add(
        style("TextBox.todo-no-clear:focus /template/ Button#DeleteButton")
            .set(Visual::is_visible_property(), false)
            .build(),
    );
}
