//! Borrowed hashtag spans for task-title presentation. Stored text is never rewritten.

use std::ops::Range;

/// Finds `#` followed by a Unicode letter, number or underscore at a word boundary.
/// Unicode identifier continuations retain combining marks inside a tag. URL
/// fragments, `C#`, Markdown headings and bare `#` are ordinary text. The ranges
/// are UTF-8 byte offsets and can be used directly to slice the original title.
pub fn ranges(text: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    let mut characters = text.char_indices().peekable();
    let mut previous = None;
    let mut url_token = false;
    let mut scheme_punctuation = 0;
    std::iter::from_fn(move || {
        while let Some((start, character)) = characters.next() {
            if character.is_whitespace() {
                url_token = false;
            }
            scheme_punctuation = match (scheme_punctuation, character) {
                (_, ':') => 1,
                (1, '/') => 2,
                (2, '/') => {
                    url_token = true;
                    0
                }
                _ => 0,
            };
            let boundary = previous.is_none_or(|previous: char| {
                !unicode_ident::is_xid_continue(previous)
                    && !matches!(previous, '#' | '/' | ':' | '.' | '?' | '&' | '=' | '%')
            });
            previous = Some(character);
            if character != '#' || !boundary || url_token {
                continue;
            }
            if !characters
                .peek()
                .is_some_and(|(_, next)| next.is_alphanumeric() || *next == '_')
            {
                continue;
            }
            let mut end = start + 1;
            while let Some(&(offset, next)) = characters.peek() {
                if !unicode_ident::is_xid_continue(next) && !next.is_alphanumeric() {
                    break;
                }
                characters.next();
                end = offset + next.len_utf8();
                previous = Some(next);
            }
            return Some(start..end);
        }
        None
    })
}
