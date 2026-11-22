//! Shared native font registration. Hosts supply bytes; the browser keeps the font outside WASM.

use ferroui_base::media::fonts::{FontCollectionBase, FontCollectionBaseImpl};
use ferroui_base::media::{
    FontFallback, FontFamily, FontManager, FontManagerOptions, UnicodeRange,
};
use std::io::Cursor;
use std::rc::Rc;
use url::Url;

pub const EMOJI_FONT_FAMILY: &str = "fonts:ToDoEmoji#Noto Color Emoji";

pub fn font_options() -> FontManagerOptions {
    FontManagerOptions {
        default_family_name: Some("fonts:Inter#Inter".into()),
        // Fallback is consulted only for missing glyphs. The complete cmap includes emoji,
        // selectors, modifiers, keycaps, and tag sequences without a hand-maintained range list.
        font_fallbacks: Some(vec![FontFallback {
            font_family: FontFamily::new(EMOJI_FONT_FAMILY),
            unicode_range: UnicodeRange::default_range(),
        }]),
        ..Default::default()
    }
}

/// Call from AppBuilder::configure_fonts after the host has installed its font backend.
/// The font manager owns the decoded glyph data; the supplied buffer may be dropped on return.
pub fn install_emoji_font(manager: &FontManager, bytes: &[u8]) -> Result<(), String> {
    let collection = Rc::new(EmojiFontCollection {
        base: FontCollectionBase::new(),
        key: Url::parse("fonts:ToDoEmoji").expect("a static font collection URI"),
    });
    let glyph = FontCollectionBase::try_add_glyph_typeface_from_stream(
        &*collection,
        &mut Cursor::new(bytes),
    )
    .ok_or_else(|| "The emoji font could not be loaded. Reload the application.".to_owned())?;
    if glyph.family_name() != "Noto Color Emoji"
        && glyph.typographic_family_name() != "Noto Color Emoji"
    {
        return Err("The bundled emoji font has an unexpected family name.".to_owned());
    }
    manager.add_font_collection(collection);
    Ok(())
}

struct EmojiFontCollection {
    base: FontCollectionBase,
    key: Url,
}

impl FontCollectionBaseImpl for EmojiFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(this: &Self) -> Url {
        this.key.clone()
    }
}
