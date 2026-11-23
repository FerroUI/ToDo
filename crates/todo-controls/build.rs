fn main() {
    // Explicit roots also retain templates used by the downstream todo-ui crate.
    // The generated table retains only these controls and their dependencies.
    ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .theme("WinUITheme")
        .theme_roots(&[
            "Window",
            "Button",
            "ToggleButton",
            "ToggleSwitch",
            "TextBox",
            "RichEditBox",
            "TextBlock",
            "PathIcon",
            "ScrollViewer",
            "ListBox",
            "ListBoxItem",
            "CheckBox",
            "RadioButton",
            "ComboBox",
            "ComboBoxItem",
            "ToolTip",
            "ContextMenu",
            "MenuItem",
            "Separator",
            "Calendar",
            "CalendarDatePicker",
            "ProgressRing",
        ])
        .run();
}
