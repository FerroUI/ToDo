use ferroui_base::media::text_formatting::unicode::{Codepoint, GraphemeEnumerator};
use ferroui_view::prelude::*;
use todo_controls::{Icon, icon};

pub(super) fn view(name: &str, color: &str) -> (Element, String) {
    let (emoji, label) = split_name(name);
    let icon = if let Some(emoji) = emoji {
        text_block()
            .text(emoji)
            .font_family(ferroui_base::media::FontFamily::new(
                todo_controls::EMOJI_FONT_FAMILY,
            ))
            .font_size(20.0)
            .foreground(color)
            .into()
    } else {
        icon(Icon::List, 20.0, color)
    };
    (icon, label.to_owned())
}

/// The leading emoji occupies the icon column; retain the original name in the model and AX label.
fn split_name(name: &str) -> (Option<String>, &str) {
    let Some(emoji) = leading_emoji(name) else {
        return (None, name);
    };
    let label = name[emoji.len()..].trim_start();
    (Some(emoji), label)
}

/// Reuse the framework's Unicode tables and grapheme rules; never split a flag or ZWJ family.
fn leading_emoji(name: &str) -> Option<String> {
    let first = Codepoint::new(name.chars().next()? as u32);
    if !first.is_emoji() {
        return None;
    }
    let units: Vec<_> = name.encode_utf16().collect();
    let grapheme = GraphemeEnumerator::new(&units).next()?;
    let units = &units[..grapheme.length()];
    if units.contains(&0xFE0E) {
        return None;
    }
    // Unicode also marks bare digits, # and * as Emoji because they can form keycaps.
    if !first.has_emoji_presentation() && !units.contains(&0xFE0F) && !units.contains(&0x20E3) {
        return None;
    }
    String::from_utf16(units).ok()
}

#[cfg(test)]
mod tests {
    use super::{leading_emoji, split_name};
    #[test]
    fn whole_emoji_clusters_and_text_fallback() {
        for emoji in ["🧑🏽‍💻", "🇵🇱", "👨‍👩‍👧‍👦", "1️⃣", "❤️"]
        {
            assert_eq!(leading_emoji(&format!("{emoji} List")), Some(emoji.into()));
        }
        for name in ["Tasks", "2026 plans", "#todo", "* Inbox", "❤︎ Text", ""] {
            assert_eq!(leading_emoji(name), None);
        }
    }

    #[test]
    fn sidebar_label_omits_only_the_icon_and_its_separator() {
        for (name, emoji, label) in [
            ("🧑🏽‍💻 \t Development", "🧑🏽‍💻", "Development"),
            ("🇵🇱Poland", "🇵🇱", "Poland"),
            ("📚\u{a0} Books", "📚", "Books"),
            ("🌿 🍃 Garden ", "🌿", "🍃 Garden "),
            ("📚", "📚", ""),
        ] {
            assert_eq!(split_name(name), (Some(emoji.into()), label));
        }
        for name in [
            "Tasks",
            "2026 plans",
            "#todo",
            "* Inbox",
            "❤︎ Text",
            " plain ",
            "",
        ] {
            assert_eq!(split_name(name), (None, name));
        }
    }
}
