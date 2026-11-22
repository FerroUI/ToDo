//! Reusable To Do controls: WinUI-based templates, semantic vector icons and theme tokens.
//!
//! Views remain declarative Rust. Controls retain FerroUI keyboard, focus, routed
//! input, automation, property binding and template infrastructure.

mod buttons;
mod fonts;
mod icons;
mod list_styles;
mod palette;
mod templates;
mod theme;

pub use buttons::{
    action_button, colored_icon_button, icon_button, primary_button, star_button, task_card,
    task_check, text, text_button, toolbar_button,
};
pub use fonts::{EMOJI_FONT_FAMILY, font_options, install_emoji_font};
pub use icons::{Icon, icon, inherited_icon};
pub use palette::{ALL_LIST_THEMES, ListTheme, Palette};
pub use theme::{install_theme, set_dark_mode};

ferroui_markup_xaml::include_compiled_xaml!();
