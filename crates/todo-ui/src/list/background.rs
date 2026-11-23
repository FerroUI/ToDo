use todo_state::AppViewModel;
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
