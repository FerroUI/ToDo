use super::*;
use ferroui_base::media::fonts::testing::{TestFontBuilder, TestFontManagerImpl, TestFontScope};
use ferroui_base::media::text_formatting::{GlyphInfo, TextShaperOptions};
use ferroui_base::media::{FontStretch, FontStyle, FontWeight, GlyphTypeface, Typeface};
use ferroui_base::platform::ITextShaperImpl;
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::FerroLocator;
use ferroui_harfbuzz::HarfBuzzTextShaper;

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

fn shaping_scope() -> (TestFontScope, Rc<HarfBuzzTextShaper>) {
    let scope = scope();
    let shaper = Rc::new(HarfBuzzTextShaper::new());
    FerroLocator::current_mutable()
        .bind::<dyn ITextShaperImpl>()
        .to_constant(shaper.clone());
    (scope, shaper)
}

fn shape(
    shaper: &HarfBuzzTextShaper,
    typeface: &Rc<GlyphTypeface>,
    text: &str,
    size: f64,
) -> Vec<GlyphInfo> {
    let text = ReadOnlyMemory::from_vec(text.encode_utf16().collect());
    let options = TextShaperOptions::with_all(typeface.clone(), size, 0, None, 0.0, 0.0, None);
    shaper
        .shape_text(&text, &options)
        .glyph_infos()
        .iter()
        .copied()
        .collect()
}

fn assert_fullwidth_nonoverlapping_run(typeface: &GlyphTypeface, glyphs: &[GlyphInfo], size: f64) {
    let scale = size / f64::from(typeface.metrics().design_em_height);
    let mut pen = 0.0;
    let mut previous_ink_right = f64::NEG_INFINITY;
    for glyph in glyphs {
        assert_ne!(
            glyph.glyph_index, 0,
            "the run must not contain a missing glyph"
        );
        assert!(
            (glyph.glyph_advance - size).abs() < 0.000_001,
            "glyph {} advanced {}px at {size}px; every CJK syllable must advance a full em",
            glyph.glyph_index,
            glyph.glyph_advance,
        );
        let metrics = typeface.try_get_glyph_metrics(glyph.glyph_index).unwrap();
        let ink_left = pen + glyph.glyph_offset.x + f64::from(metrics.x_bearing) * scale;
        let ink_right = ink_left + f64::from(metrics.width) * scale;
        assert!(
            ink_left >= previous_ink_right - 0.000_001,
            "glyph {} overlaps the preceding CJK outline: {ink_left} < {previous_ink_right}",
            glyph.glyph_index,
        );
        previous_ink_right = ink_right;
        pen += glyph.glyph_advance;
    }
    assert!((pen - glyphs.len() as f64 * size).abs() < 0.000_001);
}

#[test]
fn hangul_harfbuzz_shapes_nfc_and_nfd_with_fullwidth_advances() {
    let (_scope, shaper) = shaping_scope();
    let manager = FontManager::current();
    install_cjk_font(&manager, CJK).unwrap();
    let base = FontFamily::new("fonts:Inter#Inter");
    // The actual pinned HarfBuzz backend, not the cmap or the test platform's
    // scalar advances: the upstream composite syllables have misleading hmtx
    // widths even though their glyphs and USE_MY_METRICS outlines are complete.
    for (style, weight) in [
        (FontStyle::Normal, FontWeight::Normal),
        (FontStyle::Italic, FontWeight::Bold),
    ] {
        let matched = manager
            .try_match_character(
                '한' as i32,
                style,
                weight,
                FontStretch::Normal,
                Some(&base),
                None,
            )
            .unwrap();
        let typeface = matched.glyph_typeface();
        assert_eq!(typeface.family_name(), "WenQuanYi Micro Hei");
        for size in [14.0, 20.0] {
            let nfc = shape(&shaper, &typeface, "한국어", size);
            let nfd = shape(
                &shaper,
                &typeface,
                "\u{1112}\u{1161}\u{11ab}\u{1100}\u{116e}\u{11a8}\u{110b}\u{1165}",
                size,
            );
            assert_eq!(nfc.len(), 3);
            assert_eq!(nfd.len(), 3, "decomposed Jamo must form three syllables");
            assert_eq!(
                nfc.iter()
                    .map(|glyph| glyph.glyph_index)
                    .collect::<Vec<_>>(),
                nfd.iter()
                    .map(|glyph| glyph.glyph_index)
                    .collect::<Vec<_>>(),
                "canonical-equivalent text must use identical syllable glyphs",
            );
            assert_eq!(
                nfd.iter()
                    .map(|glyph| glyph.glyph_cluster)
                    .collect::<Vec<_>>(),
                [0, 3, 6],
            );
            assert_fullwidth_nonoverlapping_run(&typeface, &nfc, size);
            assert_fullwidth_nonoverlapping_run(&typeface, &nfd, size);
        }
    }
}

#[test]
fn hangul_harfbuzz_keeps_mixed_cjk_and_spaced_syllables_separate() {
    let (_scope, shaper) = shaping_scope();
    let typeface = cjk_glyphs();
    let cmap = typeface.character_to_glyph_map();
    for size in [14.0, 20.0] {
        let mixed = shape(&shaper, &typeface, "日本語中文한국어한글", size);
        assert_eq!(mixed.len(), 10);
        assert_fullwidth_nonoverlapping_run(&typeface, &mixed, size);

        let spaced = shape(&shaper, &typeface, "한 국 어", size);
        assert_eq!(spaced.len(), 5);
        for index in [0, 2, 4] {
            assert_fullwidth_nonoverlapping_run(&typeface, &spaced[index..=index], size);
        }
        let space = cmap.get_glyph(' ' as i32);
        let space_advance = f64::from(typeface.try_get_horizontal_glyph_advance(space).unwrap())
            * size
            / f64::from(typeface.metrics().design_em_height);
        assert!(space_advance > 0.0);
        for index in [1, 3] {
            assert_eq!(spaced[index].glyph_index, space);
            assert!((spaced[index].glyph_advance - space_advance).abs() < 0.000_001);
        }
        assert!(
            (spaced.iter().map(|glyph| glyph.glyph_advance).sum::<f64>()
                - (3.0 * size + 2.0 * space_advance))
                .abs()
                < 0.000_001,
        );
    }
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
