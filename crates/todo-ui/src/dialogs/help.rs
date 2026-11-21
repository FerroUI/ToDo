use ferroui_base::media::TextWrapping;
use ferroui_view::prelude::*;

pub fn view() -> Element {
    let mut content = stack_panel().spacing(12.0).margin(8.0);
    for (key, action) in [
        ("Ctrl / ⌘ + N", "Focus Add a task"),
        ("Ctrl / ⌘ + L", "Create a list"),
        ("Ctrl / ⌘ + F", "Search tasks"),
        ("Ctrl / ⌘ + A", "Select all visible tasks"),
        ("Ctrl / ⌘ + click", "Select or deselect a task"),
        ("Shift + click", "Select a range of tasks"),
        ("Up / Down", "Move focus between tasks"),
        ("Shift + Up / Down", "Extend the selection"),
        ("Home / End", "Focus the first or last task"),
        ("Enter", "Open a task; submit a task or step while typing"),
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
        ("Escape", "Close a menu, details, or task selection"),
    ] {
        content = content.child(
            grid()
                .column_definitions("145,*")
                .child(text_block().text(key).font_size(12.0))
                .child(
                    text_block()
                        .grid_column(1)
                        .text(action)
                        .font_size(12.0)
                        .text_wrapping(TextWrapping::Wrap),
                ),
        );
    }
    content.into()
}
