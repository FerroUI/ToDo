//! WinUI base with app resources and lightweight, fully templated action controls.

use crate::compiled_markup::THEME_PARTS_FERROUI_THEMES_WINUI_WINUITHEME as THEME_PARTS;
use crate::palette::Palette;
use crate::templates::flat_button_theme;
use ferroui_base::controls::ResourceDictionary;
use ferroui_base::layout::{HorizontalAlignment, Layoutable, VerticalAlignment};
use ferroui_base::media::{Color, IBrush, SolidColorBrush};
use ferroui_base::styling::{Setter, Style, ThemeVariant};
use ferroui_base::{FerroProperty, Ref, StyledElement, Visual};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{Application, Border, ContentControl, TextBlock};
use ferroui_markup_xaml::markup_extensions::DynamicResourceExtension;
use ferroui_view::style;
use std::rc::Rc;

/// Install once after platform services are initialized and before mounting views.
pub fn install_theme(app: &Application) {
    app.styles()
        .add(ferroui_themes_winui::WinUITheme::with_parts(None, THEME_PARTS).as_style());
    for (variant, palette) in [
        (ThemeVariant::light(), Palette::light()),
        (ThemeVariant::dark(), Palette::dark()),
    ] {
        let resources = ResourceDictionary::new();
        for (key, value) in [
            ("TodoBackgroundBrush", palette.background),
            ("TodoSidebarBrush", palette.sidebar),
            ("TodoSurfaceBrush", palette.surface),
            ("TodoHoverBrush", palette.surface_hover),
            ("TodoTextBrush", palette.text),
            ("TodoSecondaryBrush", palette.secondary),
            ("TodoBorderBrush", palette.border),
            ("TodoAccentBrush", palette.accent),
            ("TodoSelectionBrush", palette.selection),
        ] {
            let brush: Rc<dyn IBrush> =
                SolidColorBrush::with_color(Color::parse(value).expect("valid theme color")).into();
            resources.add_value(key, brush);
        }
        app.resources().add_theme_dictionary(variant, &resources);
    }
    install_buttons(app);
    crate::list_styles::install(app);
    crate::input_styles::install(app);
    let body = style("TextBlock")
        .set(TextBlock::font_size_property(), 14.0)
        .build();
    app.styles().add(body);
    let secondary = style("TextBlock.todo-secondary").build();
    bind(
        &secondary,
        TextBlock::foreground_property().as_property(),
        "TodoSecondaryBrush",
    );
    app.styles().add(secondary);
    app.styles().add(
        style("TextBox.todo-input")
            .set(TemplatedControl::border_thickness_property(), 0.0)
            .set(TemplatedControl::background_property(), "Transparent")
            .set(TemplatedControl::font_size_property(), 14.0)
            .set(TemplatedControl::corner_radius_property(), 4.0)
            .build(),
    );
    let card = style("Border.todo-task")
        .set(Border::corner_radius_property(), 4.0)
        .build();
    bind(
        &card,
        Border::background_property().as_property(),
        "TodoSurfaceBrush",
    );
    app.styles().add(card);
}

pub fn set_dark_mode(app: &Application, dark: bool) {
    app.set_requested_theme_variant(Some(if dark {
        ThemeVariant::dark()
    } else {
        ThemeVariant::light()
    }));
}

fn install_buttons(app: &Application) {
    for selector in [
        "Button.todo-flat",
        "Button.todo-toolbar",
        "Button.todo-nav",
        "Button.todo-action",
        "Button.todo-task",
        "Button.todo-primary",
        "CheckBox.todo-completion",
        "ToggleButton.todo-star",
    ] {
        let base = style(&selector)
            .set(StyledElement::theme_property(), Some(flat_button_theme()))
            .set(TemplatedControl::background_property(), "Transparent")
            .set(TemplatedControl::border_brush_property(), "Transparent")
            .set(TemplatedControl::border_thickness_property(), 1.0)
            .set(TemplatedControl::corner_radius_property(), 4.0)
            .set(TemplatedControl::padding_property(), (8.0, 6.0))
            .set(TemplatedControl::font_size_property(), 14.0)
            .set(Layoutable::min_height_property(), 32.0)
            .set(
                ContentControl::horizontal_content_alignment_property(),
                HorizontalAlignment::Center,
            )
            .set(
                ContentControl::vertical_content_alignment_property(),
                VerticalAlignment::Center,
            )
            .build();
        bind(
            &base,
            TemplatedControl::foreground_property().as_property(),
            "TodoTextBrush",
        );
        app.styles().add(base);
        for (pseudo, resource) in [
            (":pointerover", "TodoHoverBrush"),
            (":pressed", "TodoSelectionBrush"),
        ] {
            let state = style(&format!("{selector}{pseudo}")).build();
            bind(
                &state,
                TemplatedControl::background_property().as_property(),
                resource,
            );
            app.styles().add(state);
        }
        let focus = style(&format!("{selector}:focus-visible")).build();
        bind(
            &focus,
            TemplatedControl::border_brush_property().as_property(),
            "TodoAccentBrush",
        );
        app.styles().add(focus);
        app.styles().add(
            style(&format!("{selector}:disabled"))
                .set(Visual::opacity_property(), 0.45)
                .build(),
        );
    }
    for (state, color) in [(":pointerover", "#26FFFFFF"), (":pressed", "#40FFFFFF")] {
        app.styles().add(
            style(&format!("Button.todo-on-color{state}"))
                .set(TemplatedControl::background_property(), color)
                .build(),
        );
    }
    let selected = style("Button.todo-nav.selected").build();
    bind(
        &selected,
        TemplatedControl::background_property().as_property(),
        "TodoSelectionBrush",
    );
    bind(
        &selected,
        TemplatedControl::foreground_property().as_property(),
        "TodoAccentBrush",
    );
    app.styles().add(selected);
    let action = style("Button.todo-action").build();
    bind(
        &action,
        TemplatedControl::foreground_property().as_property(),
        "TodoAccentBrush",
    );
    app.styles().add(action);
    app.styles().add(
        style("Button.todo-primary")
            .set(TemplatedControl::background_property(), "#2564CF")
            .set(TemplatedControl::foreground_property(), "#FFFFFF")
            .set(TemplatedControl::padding_property(), (16.0, 7.0))
            .build(),
    );
    app.styles().add(
        style("Button.todo-primary:pointerover")
            .set(TemplatedControl::background_property(), "#1E56B5")
            .build(),
    );
    app.styles().add(
        style("Button.todo-primary:pressed")
            .set(TemplatedControl::background_property(), "#18489C")
            .build(),
    );
}

fn bind(style: &Ref<Style>, property: &'static FerroProperty, resource: &'static str) {
    let binding = DynamicResourceExtension::with_resource_key(Some(Rc::new(resource.to_owned())));
    style.add_setter(Setter::new_binding_base(property, binding.as_binding()));
}
