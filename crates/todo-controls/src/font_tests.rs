use super::*;
use ferroui_base::media::fonts::testing::{TestFontBuilder, TestFontManagerImpl, TestFontScope};
use ferroui_base::media::{FontStretch, FontStyle, FontWeight, GlyphTypeface, Typeface};
use ferroui_base::FerroLocator;

const CJK: &[u8] = include_bytes!("../assets/fonts/WenQuanYiMicroHei.ttf");
const EMOJI: &[u8] = include_bytes!("../assets/fonts/Noto-COLRv1.ttf");

fn scope() -> TestFontScope {
    // No installed/system fonts: this is the browser's fallback environment.
    let scope = TestFontScope::with_font_manager(TestFontManagerImpl::new("Inter"));
    FerroLocator::current_mutable()
        .bind::<FontManagerOptions>()
        .to_constant(Rc::new(font_options()));
    let manager = FontManager::current();
    install_font(
        &manager,
        &TestFontBuilder::new("Inter")
            .codepoints(&[(0x20, 0x7e)])
            .build_bytes(),
        "fonts:Inter",
        "Inter",
        "test base",
    )
    .unwrap();
    scope
}

fn cjk_glyphs() -> Rc<GlyphTypeface> {
    let manager = FontManager::current();
    install_cjk_font(&manager, CJK).unwrap();
    manager
        .try_get_glyph_typeface(&Typeface::from_name(CJK_FONT_FAMILY))
        .unwrap()
}

#[test]
fn complete_cjk_face_retains_documented_han_hangul_and_kana_coverage() {
    let _scope = scope();
    let glyphs = cjk_glyphs();
    assert_eq!(glyphs.family_name(), "WenQuanYi Micro Hei");
    assert_eq!(glyphs.glyph_count(), 49_531);
    let cmap = glyphs.character_to_glyph_map();
    for (first, last) in [(0x4e00, 0x9fc3), (0xac00, 0xd7a3), (0xff01, 0xff5e)] {
        for codepoint in first..=last {
            assert!(
                cmap.try_get_glyph(codepoint)
                    .is_some_and(|glyph| glyph != 0),
                "U+{codepoint:04X}"
            );
        }
    }
    for character in "日本語 ひらがな カタカナ 简体中文 繁體中文 한국어 한글".chars()
    {
        assert!(
            cmap.try_get_glyph(character as i32)
                .is_some_and(|glyph| glyph != 0),
            "{character}"
        );
    }
}

#[test]
fn mixed_text_resolves_to_registered_cjk_and_color_emoji_without_system_fonts() {
    let scope = scope();
    let manager = FontManager::current();
    install_emoji_font(&manager, EMOJI).unwrap();
    install_cjk_font(&manager, CJK).unwrap();
    let base = FontFamily::new("fonts:Inter#Inter");
    for (style, weight) in [
        (FontStyle::Normal, FontWeight::Normal),
        (FontStyle::Italic, FontWeight::Bold),
    ] {
        for (text, expected) in [
            (
                "日本語かなカナ简体中文繁體中文한국어",
                "WenQuanYi Micro Hei",
            ),
            ("👩🏽💻", "Noto Color Emoji"),
        ] {
            for character in text.chars() {
                let matched = manager
                    .try_match_character(
                        character as i32,
                        style,
                        weight,
                        FontStretch::Normal,
                        Some(&base),
                        None,
                    )
                    .unwrap_or_else(|| panic!("missing fallback for {character}"));
                let glyphs = matched.glyph_typeface();
                assert_eq!(glyphs.family_name(), expected, "{character}");
                assert!(glyphs
                    .character_to_glyph_map()
                    .try_get_glyph(character as i32)
                    .is_some_and(|glyph| glyph != 0));
            }
        }
    }
    // The ordinary requested face still resolves directly, rather than changing the UI font.
    assert_eq!(
        manager
            .try_get_glyph_typeface(&Typeface::from_name("fonts:Inter#Inter"))
            .unwrap()
            .family_name(),
        "Inter"
    );
    assert_eq!(scope.font_manager_impl().match_character_calls(), 0);
}

#[test]
fn invalid_or_wrong_family_cjk_bytes_are_rejected_before_registration() {
    let _scope = scope();
    let manager = FontManager::current();
    assert!(install_cjk_font(&manager, b"not a font")
        .unwrap_err()
        .contains("could not be loaded"));
    assert!(install_cjk_font(&manager, EMOJI)
        .unwrap_err()
        .contains("unexpected family name"));
    assert!(manager
        .try_get_glyph_typeface(&Typeface::from_name(CJK_FONT_FAMILY))
        .is_none());
    install_cjk_font(&manager, CJK).unwrap();
    assert!(manager
        .try_get_glyph_typeface(&Typeface::from_name(CJK_FONT_FAMILY))
        .is_some());
}
