use crate::palette;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_state::AppViewModel;

pub fn view(vm: Rc<AppViewModel>) -> Element {
    let p = palette(&vm);
    let compact = vm.is_narrow();
    let new_task = if cfg!(target_arch = "wasm32") {
        "Ctrl / ⌘ + Alt + N"
    } else {
        "Ctrl / ⌘ + N"
    };
    let mut content = stack_panel().spacing(14.0).margin(8.0);
    for (key, action) in [
        (new_task, "Focus Add a task"),
        ("Ctrl / ⌘ + L", "Create a list"),
        ("Ctrl / ⌘ + F", "Search tasks"),
        ("Ctrl / ⌘ + A", "Select all visible tasks"),
        ("Ctrl / ⌘ + click", "Select or deselect a task"),
        ("Shift + click", "Select a range of tasks"),
        ("Up / Down", "Move focus between tasks"),
        ("Shift + Up / Down", "Extend the selection"),
        ("Home / End", "Focus the first or last task"),
        ("Enter", "Open a task; submit a task or step while typing"),
        ("Ctrl / ⌘ + Enter", "Save a note"),
        (
            "Space",
            "Complete the focused task, or toggle its selection",
        ),
        ("Shift + F10", "Open task options"),
        ("Delete", "Delete the selected tasks"),
        ("Ctrl / ⌘ + D", "Complete selected tasks"),
        ("Ctrl / ⌘ + T", "Add the focused task to My Day"),
        ("Ctrl / ⌘ + Z", "Undo"),
        ("Ctrl / ⌘ + Y", "Redo"),
        ("Ctrl / ⌘ + R", "Refresh tasks"),
        ("Alt + Left", "Close the current view or go back"),
        ("Page Up / Page Down", "Move between months in the calendar"),
        ("Escape", "Close a menu, details, or task selection"),
    ] {
        content = content.child(
            grid()
                .column_definitions(if compact { "*" } else { "132,*" })
                .row_definitions(if compact { "Auto,Auto" } else { "Auto" })
                .column_spacing(12.0)
                .row_spacing(4.0)
                .child(
                    text_block()
                        .text(key)
                        .font_size(13.0)
                        .font_weight(FontWeight::SemiBold)
                        .foreground(p.text)
                        .text_wrapping(TextWrapping::Wrap),
                )
                .child(
                    text_block()
                        .grid_column(if compact { 0 } else { 1 })
                        .grid_row(if compact { 1 } else { 0 })
                        .text(action)
                        .font_size(14.0)
                        .foreground(p.secondary)
                        .text_wrapping(TextWrapping::Wrap),
                ),
        );
    }
    content.into()
}
