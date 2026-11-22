use ferroui_base::media::text_formatting::unicode::{Codepoint, GraphemeEnumerator};
use ferroui_view::prelude::*;
use todo_controls::{Icon, icon};

pub(super) fn view(name: &str, color: &str) -> Element {
    if let Some(emoji) = leading_emoji(name) {
        text_block()
            .text(emoji)
            .font_size(20.0)
            .foreground(color)
            .into()
    } else {
        icon(Icon::List, 20.0, color)
    }
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
    use super::leading_emoji;
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
}
