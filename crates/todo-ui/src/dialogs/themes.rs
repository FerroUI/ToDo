use crate::palette;
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_controls::{Control, TopLevel};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{Palette, ALL_LIST_THEMES};
use todo_domain::View;
use todo_state::{AppViewModel, Panel};

pub(super) fn view(vm: Rc<AppViewModel>) -> Element {
    let p = palette(&vm);
    let selected_accent = vm.accent();
    let mut swatches = wrap_panel()
        .orientation(Orientation::Horizontal)
        .item_width(104.0)
        .item_height(76.0);
    if !ALL_LIST_THEMES
        .iter()
        .any(|theme| selected_accent.eq_ignore_ascii_case(theme.accent))
    {
        let keep = vm.clone();
        swatches = swatches.child(swatch(
            "Current",
            &selected_accent,
            p.surface_hover,
            true,
            p,
            // Keeping a custom/default color must preserve its background too.
            move || keep.show_panel(Panel::None),
        ));
    }
    for theme in ALL_LIST_THEMES {
        let select = vm.clone();
        let color = theme.accent.to_string();
        swatches = swatches.child(swatch(
            theme.name,
            theme.accent,
            if vm.is_dark() {
                theme.dark_background
            } else {
                theme.background
            },
            selected_accent.eq_ignore_ascii_case(theme.accent),
            p,
            move || {
                select.set_smart_color(color.clone());
                select.show_panel(Panel::None);
            },
        ));
    }
    let selected = vm
        .wallpaper()
        .or_else(|| matches!(vm.view(), View::MyDay).then(|| "scene:mountains".into()));
    let mut gallery = wrap_panel()
        .orientation(Orientation::Horizontal)
        .item_width(94.0)
        .item_height(80.0);
    for scene in crate::list::wallpaper::SCENES {
        let select = vm.clone();
        let id = scene.id;
        gallery = gallery.child(
            button()
                .name(format!("Wallpaper{}", scene.name.replace(' ', "")))
                .class("todo-flat")
                .width(86.0)
                .height(72.0)
                .margin(4.0)
                .padding(0.0)
                .clip_to_bounds(true)
                .border_thickness(2.0)
                .border_brush(if selected.as_deref() == Some(id) {
                    p.accent
                } else {
                    "Transparent"
                })
                .automation_properties_name(format!("{} background", scene.name))
                .automation_properties_item_status(
                    (selected.as_deref() == Some(id)).then(|| "Selected".to_owned()),
                )
                .child(
                    grid()
                        .child(
                            crate::list::wallpaper::view(id, vm.is_dark())
                                .unwrap_or(Element::Empty),
                        )
                        .child(
                            border()
                                .vertical_alignment(ferroui_base::layout::VerticalAlignment::Bottom)
                                .background("#99000000")
                                .padding(3.0)
                                .child(
                                    text_block()
                                        .text(scene.name)
                                        .font_size(11.0)
                                        .foreground("#FFFFFF"),
                                ),
                        ),
                )
                .on_click(move |event| {
                    select.set_wallpaper(Some(id.into()));
                    select.show_panel(Panel::None);
                    event.set_handled(true);
                }),
        );
    }
    let custom = vm.clone();
    let clear = vm.clone();
    stack_panel()
        .spacing(12.0)
        .child(
            text_block()
                .text("Choose a color for this list")
                .font_size(13.0)
                .foreground(p.secondary)
                .margin(8.0),
        )
        .child(swatches)
        .child(text_block().text("Original backgrounds").font_size(13.0).foreground(p.secondary).margin((8.0, 6.0)))
        .child(gallery)
        .child(wrap_panel().orientation(Orientation::Horizontal)
            .child(button().name("ChooseBackgroundImage").content("Choose image…").margin(4.0)
                .automation_properties_name("Choose a custom background image")
                .on_click(move |event| {
                    if let Some(root) = event.source().and_then(|source| source.cast::<Control>()).and_then(|source| TopLevel::get_top_level(Some(&source))) {
                        super::background_image::choose(custom.clone(), root);
                    }
                    event.set_handled(true);
                }))
            .child(button().name("ClearBackground").content("No background").margin(4.0)
                .on_click(move |event| { clear.set_wallpaper(Some("none".into())); clear.show_panel(Panel::None); event.set_handled(true); })))
        .child(text_block().text("JPEG or PNG, up to 8 MiB and 12 megapixels. A resized copy travels with this list in backups and device settings. Custom backgrounds share a 2 MiB document budget.")
            .font_size(12.0).foreground(p.secondary).text_wrapping(ferroui_base::media::TextWrapping::Wrap).margin(8.0))
        .into()
}

fn swatch(
    name: &str,
    accent: &str,
    background: &str,
    selected: bool,
    p: Palette,
    choose: impl Fn() + 'static,
) -> Element {
    button()
        .class("todo-flat")
        .width(94.0)
        .height(68.0)
        .margin(4.0)
        .padding(0.0)
        .border_thickness(2.0)
        .border_brush(if selected { p.accent } else { "Transparent" })
        .automation_properties_name(format!("{name} list theme"))
        .automation_properties_item_status(selected.then(|| "Selected".to_string()))
        .tool_tip_tip(name)
        .on_click(move |_| choose())
        .child(
            border()
                .background(background)
                .corner_radius(4.0)
                .padding(10.0)
                .child(
                    stack_panel()
                        .spacing(7.0)
                        .child(
                            border()
                                .height(5.0)
                                .width(54.0)
                                .horizontal_alignment(H::Left)
                                .background(accent)
                                .corner_radius(2.0),
                        )
                        .child(text_block().text(name).font_size(12.0).foreground(p.text)),
                ),
        )
        .into()
}
