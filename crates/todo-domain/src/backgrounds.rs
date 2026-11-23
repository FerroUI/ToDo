//! Portable wallpapers: original scene identifiers or bounded, self-contained JPEGs.
use base64::{Engine, engine::general_purpose::STANDARD};

pub const MAX_BACKGROUND_BYTES: usize = 192 * 1024;
pub const MAX_BACKGROUND_SIDE: u32 = 1200;
/// Leave room for task presentation records in the 4 MiB OneDrive replica and
/// for task/attachment metadata in a portable document.
pub const MAX_DOCUMENT_BACKGROUND_BYTES: usize = 2 * 1024 * 1024;
pub const JPEG_PREFIX: &str = "data:image/jpeg;base64,";
pub const SCENES: [&str; 6] = [
    "scene:mountains",
    "scene:coast",
    "scene:forest",
    "scene:dunes",
    "scene:lake",
    "scene:night",
];

pub fn valid_background(value: &str) -> bool {
    if value.starts_with("data:") {
        return decode_background(value).is_ok();
    }
    // Keep old provider-specific short identifiers inert and round-trippable.
    !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
}

pub fn validate_collection<'a>(
    values: impl IntoIterator<Item = &'a str>,
) -> Result<(), &'static str> {
    let mut total = 0usize;
    for value in values {
        total = total.saturating_add(value.len());
        if total > MAX_DOCUMENT_BACKGROUND_BYTES {
            return Err(
                "Backgrounds exceed the 2 MiB document limit. Remove a custom background before adding another.",
            );
        }
        if !valid_background(value) {
            return Err("A background is invalid or too large");
        }
    }
    Ok(())
}

pub fn valid_smart_backgrounds(values: &std::collections::BTreeMap<String, String>) -> bool {
    values.len() <= 64
        && values
            .keys()
            .all(|key| !key.is_empty() && key.len() <= 128 && !key.chars().any(char::is_control))
}

pub fn decode_background(value: &str) -> Result<Vec<u8>, &'static str> {
    let encoded = value
        .strip_prefix(JPEG_PREFIX)
        .ok_or("Choose a supported background image")?;
    if encoded.len() > MAX_BACKGROUND_BYTES.div_ceil(3) * 4 {
        return Err("Background image exceeds 192 KiB");
    }
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| "Background image encoding is invalid")?;
    if bytes.len() > MAX_BACKGROUND_BYTES || !bytes.starts_with(&[0xff, 0xd8]) {
        return Err("Background image is invalid or too large");
    }
    let (width, height) =
        image_dimensions(&bytes).ok_or("Background image dimensions are invalid")?;
    if width > MAX_BACKGROUND_SIDE || height > MAX_BACKGROUND_SIDE {
        return Err("Background image exceeds 1200 pixels");
    }
    Ok(bytes)
}

pub fn encode_background(bytes: &[u8]) -> Result<String, &'static str> {
    let value = format!("{JPEG_PREFIX}{}", STANDARD.encode(bytes));
    decode_background(&value)?;
    Ok(value)
}

/// Inspect dimensions before a renderer allocates decoded pixels. Input accepts
/// PNG/JPEG only; GIF/SVG/remote URLs never enter the wallpaper decoder.
pub fn image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let dimensions =
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") && bytes.get(8..16) == Some(b"\0\0\0\rIHDR") {
            (
                u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?),
                u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?),
            )
        } else if bytes.starts_with(&[0xff, 0xd8]) {
            let mut position = 2;
            loop {
                if *bytes.get(position)? != 0xff {
                    return None;
                }
                while bytes.get(position) == Some(&0xff) {
                    position += 1;
                }
                let marker = *bytes.get(position)?;
                position += 1;
                if marker == 0xda || marker == 0xd9 {
                    return None;
                }
                if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
                    continue;
                }
                let length = u16::from_be_bytes(bytes.get(position..position + 2)?.try_into().ok()?)
                    as usize;
                if length < 2 || position + length > bytes.len() {
                    return None;
                }
                if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
                    if length < 8 {
                        return None;
                    }
                    let height =
                        u16::from_be_bytes(bytes.get(position + 3..position + 5)?.try_into().ok()?)
                            as u32;
                    let width =
                        u16::from_be_bytes(bytes.get(position + 5..position + 7)?.try_into().ok()?)
                            as u32;
                    break (width, height);
                }
                position += length;
            }
        } else {
            return None;
        };
    (dimensions.0 > 0 && dimensions.1 > 0).then_some(dimensions)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dimensions_are_bounded_before_decode_and_inline_values_are_portable() {
        let jpeg = [0xff, 0xd8, 0xff, 0xc0, 0, 8, 8, 0, 32, 0, 64, 1, 0xff, 0xd9];
        assert_eq!(image_dimensions(&jpeg), Some((64, 32)));
        let encoded = encode_background(&jpeg).unwrap();
        assert_eq!(decode_background(&encoded).unwrap(), jpeg);
        assert!(valid_background("scene:mountains"));
        assert!(!valid_background("data:image/svg+xml,<svg/>"));
        let mut huge = jpeg;
        huge[9] = 0xff;
        huge[10] = 0xff;
        assert!(encode_background(&huge).is_err());
        assert_eq!(image_dimensions(&jpeg[..9]), None);
    }
}
