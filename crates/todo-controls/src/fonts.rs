//! Shared native font registration. Hosts supply bytes; the browser keeps the font outside WASM.

use ferroui_base::media::fonts::{FontCollectionBase, FontCollectionBaseImpl};
use ferroui_base::media::{
    FontFallback, FontFamily, FontManager, FontManagerOptions, UnicodeRange,
};
use std::io::Cursor;
use std::rc::Rc;
use url::Url;

pub const EMOJI_FONT_FAMILY: &str = "fonts:ToDoEmoji#Noto Color Emoji";
pub const CJK_FONT_FAMILY: &str = "fonts:ToDoCjk#WenQuanYi Micro Hei";

pub fn font_options() -> FontManagerOptions {
    FontManagerOptions {
        default_family_name: Some("fonts:Inter#Inter".into()),
        // Consult complete character maps only when the requested face lacks a glyph.
        // Keep color emoji before the text fallback where their symbol coverage overlaps.
        font_fallbacks: Some(
            [EMOJI_FONT_FAMILY, CJK_FONT_FAMILY]
                .into_iter()
                .map(|name| FontFallback {
                    font_family: FontFamily::new(name),
                    unicode_range: UnicodeRange::default_range(),
                })
                .collect(),
        ),
        ..Default::default()
    }
}

/// Call from AppBuilder::configure_fonts after the host has installed its font backend.
/// The font manager owns the decoded glyph data; the supplied buffer may be dropped on return.
pub fn install_emoji_font(manager: &FontManager, bytes: &[u8]) -> Result<(), String> {
    install_font(
        manager,
        bytes,
        "fonts:ToDoEmoji",
        "Noto Color Emoji",
        "emoji",
    )
}

/// The complete proportional Micro Hei face, shared by browser and native hosts.
pub fn install_cjk_font(manager: &FontManager, bytes: &[u8]) -> Result<(), String> {
    install_font(
        manager,
        bytes,
        "fonts:ToDoCjk",
        "WenQuanYi Micro Hei",
        "CJK",
    )
}

fn install_font(
    manager: &FontManager,
    bytes: &[u8],
    key: &str,
    family: &str,
    description: &str,
) -> Result<(), String> {
    let collection = Rc::new(BundledFontCollection {
        base: FontCollectionBase::new(),
        key: Url::parse(key).expect("a static font collection URI"),
    });
    let glyph = FontCollectionBase::try_add_glyph_typeface_from_stream(
        &*collection,
        &mut Cursor::new(bytes),
    )
    .ok_or_else(|| {
        format!("The {description} font could not be loaded. Reload the application.")
    })?;
    if glyph.family_name() != family && glyph.typographic_family_name() != family {
        return Err(format!(
            "The bundled {description} font has an unexpected family name."
        ));
    }
    manager.add_font_collection(collection);
    Ok(())
}

struct BundledFontCollection {
    base: FontCollectionBase,
    key: Url,
}

impl FontCollectionBaseImpl for BundledFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(this: &Self) -> Url {
        this.key.clone()
    }
}

#[cfg(test)]
#[path = "font_tests.rs"]
mod tests;
