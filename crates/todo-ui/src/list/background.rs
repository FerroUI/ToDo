use ferroui_view::prelude::*;
use todo_state::AppViewModel;
ferroui_base::ferro_thread_local! {
    static LANDSCAPE: std::cell::OnceCell<[ferroui_base::Ref<ferroui_base::media::Geometry>; 3]> = const { std::cell::OnceCell::new() };
}

pub(super) fn landscape(dark: bool, sky: &str) -> Element {
    use ferroui_base::media::{Geometry, Stretch};
    let geometry = LANDSCAPE.with(|cache| {
        cache
            .get_or_init(|| {
                [
                    Geometry::parse(
                        "M0 500 L170 280 300 400 470 170 680 420 850 260 1200 490 1200 800 0 800Z",
                    )
                    .expect("landscape outline"),
                    Geometry::parse(
                        "M0 560 L180 420 310 520 620 290 820 500 1050 350 1200 480 1200 800 0 800Z",
                    )
                    .expect("landscape outline"),
                    Geometry::parse(
                        "M0 640 L240 490 480 670 720 460 960 580 1200 430 1200 800 0 800Z",
                    )
                    .expect("landscape outline"),
                ]
            })
            .clone()
    });
    let colors = [
        mix_color("#FFFFFF", sky, if dark { 8 } else { 16 }),
        mix_color("#152D4A", sky, 25),
        mix_color("#10243F", sky, 48),
    ];
    let mut scene = canvas()
        .width(1200.0)
        .height(800.0)
        .is_hit_test_visible(false);
    for (data, color) in geometry.into_iter().zip(colors) {
        scene = scene.child(
            ferroui_view::prelude::path()
                .data(Some(data))
                .fill(color.as_str())
                .stretch(Stretch::None),
        );
    }
    // Viewbox owns a typed Child property; it is not a Decorator or ContentControl.
    // Mount through that property so the reconciler can retain and update the scene.
    viewbox()
        .stretch(Stretch::UniformToFill)
        .clip_to_bounds(true)
        .is_hit_test_visible(false)
        .set(ferroui_controls::Viewbox::child_property(), scene)
        .into()
}

/// The list's color also colors its completion and importance controls.
pub(crate) fn list_accent(vm: &AppViewModel) -> String {
    let color = vm.accent();
    if vm.is_dark() {
        mix_color("#FFFFFF", &color, 44)
    } else {
        color
    }
}

pub(super) fn list_background(vm: &AppViewModel, my_day: bool) -> String {
    let dark = vm.is_dark();
    let color = vm.accent();
    if my_day {
        if vm.state().settings.smart_colors.contains_key("My Day") {
            return mix_color(&color, "#202B3C", if dark { 40 } else { 76 });
        }
        return if dark { "#263249" } else { "#586B90" }.into();
    }
    if let Some(theme) = todo_controls::ALL_LIST_THEMES
        .iter()
        .find(|theme| theme.accent.eq_ignore_ascii_case(&color))
    {
        return if dark {
            theme.dark_background
        } else {
            theme.background
        }
        .into();
    }
    mix_color(
        &color,
        if dark { "#202020" } else { "#FFFFFF" },
        if dark { 18 } else { 7 },
    )
}

fn mix_color(color: &str, base: &str, weight: u32) -> String {
    let parse = |value: &str| {
        u32::from_str_radix(value.strip_prefix('#').unwrap_or(value), 16).unwrap_or(0x58699D)
    };
    let (color, base) = (parse(color), parse(base));
    let channel = |shift: u32| {
        (((color >> shift) & 255u32) * weight + ((base >> shift) & 255u32) * (100u32 - weight))
            / 100u32
    };
    format!("#{:02X}{:02X}{:02X}", channel(16), channel(8), channel(0))
}
