//! Reusable To Do controls: WinUI-based templates, semantic vector icons and theme tokens.
//!
//! Views remain declarative Rust. Controls retain FerroUI keyboard, focus, routed
//! input, automation, property binding and template infrastructure.

mod buttons;
mod icons;
mod list_styles;
mod palette;
mod templates;
mod theme;

pub use buttons::{
    action_button, colored_icon_button, icon_button, primary_button, star_button, task_card,
    task_check, text, text_button, toolbar_button,
};
pub use icons::{icon, inherited_icon, Icon};
pub use palette::{ListTheme, Palette, ALL_LIST_THEMES};
pub use theme::{install_theme, set_dark_mode};

ferroui_markup_xaml::include_compiled_xaml!();
