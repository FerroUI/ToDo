use todo_domain::hashtags;

fn tags(text: &str) -> Vec<&str> {
    hashtags::ranges(text).map(|range| &text[range]).collect()
}

#[test]
fn spans_preserve_original_unicode_and_punctuation() {
    assert_eq!(
        tags("Review (#Work), #żółć! #東京 #cafe\u{301} #शिक्षा #2026 #release_2."),
        [
            "#Work",
            "#żółć",
            "#東京",
            "#cafe\u{301}",
            "#शिक्षा",
            "#2026",
            "#release_2"
        ]
    );
}

#[test]
fn ordinary_hashes_and_url_fragments_are_not_tags() {
    assert!(
        tags("C# F# ## Heading # #\u{301} url/path#anchor https://site/#fragment ?q=#query")
            .is_empty()
    );
    assert_eq!(
        tags("#work #workshop [#work]"),
        ["#work", "#workshop", "#work"]
    );
    assert_eq!(
        tags("https://example.com/;#fragment https://example.com/+#other #actual"),
        ["#actual"]
    );
}

#[test]
fn spans_reconstruct_the_original_text_without_normalization() {
    let text = "😀 Plan #zażółć with #東京 and plain text";
    let mut reconstructed = String::new();
    let mut end = 0;
    for range in hashtags::ranges(text) {
        reconstructed.push_str(&text[end..range.start]);
        reconstructed.push_str(&text[range.clone()]);
        end = range.end;
    }
    reconstructed.push_str(&text[end..]);
    assert_eq!(reconstructed, text);
}
