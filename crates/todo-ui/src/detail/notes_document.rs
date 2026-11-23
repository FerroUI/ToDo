//! The portable note model and FerroUI's transactional UTF-16 document meet here.
use ferroui_base::media::{
    FontStyle, FontWeight, TextDecorationCollection, TextDecorationLocation, TextDecorations,
};
use ferroui_controls::documents::{
    CharacterFormat, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind, ParagraphFormat,
    ParagraphSnapshot, UnderlineStyle,
};
use ferroui_controls::rich_text::{EditError, EditKind, EditorSelection};
use ferroui_controls::RichEditBox;
use todo_domain::notes::{safe_link, NoteParagraph, NoteRun, RichNote};

const LIST_INDENT: f64 = 24.0;

pub(super) fn from_task(task: &todo_domain::Task) -> RichNote {
    task.notes_html
        .as_deref()
        .and_then(|html| RichNote::from_html(html).ok())
        .filter(|note| note.text() == task.notes)
        .unwrap_or_else(|| RichNote::plain(&task.notes))
}

pub(super) fn snapshot(note: &RichNote) -> DocumentSnapshot {
    DocumentSnapshot {
        paragraphs: note
            .paragraphs
            .iter()
            .map(|paragraph| {
                let mut inlines = Vec::new();
                if paragraph.list.is_some() {
                    inlines.push(InlineSnapshot::run(paragraph.prefix()));
                }
                for run in &paragraph.runs {
                    let children = run
                        .text
                        .split('\n')
                        .enumerate()
                        .flat_map(|(index, text)| {
                            let mut parts = Vec::new();
                            if index != 0 {
                                parts.push(InlineSnapshot {
                                    format: format(run),
                                    kind: InlineSnapshotKind::LineBreak,
                                });
                            }
                            if !text.is_empty() {
                                parts.push(InlineSnapshot {
                                    format: format(run),
                                    kind: InlineSnapshotKind::Run(text.into()),
                                });
                            }
                            parts
                        })
                        .collect::<Vec<_>>();
                    if let Some(uri) = run.link.as_deref().and_then(safe_link) {
                        inlines.push(hyperlink(children, &uri));
                    } else {
                        inlines.extend(children);
                    }
                }
                ParagraphSnapshot {
                    format: paragraph_format(paragraph.list.is_some()),
                    inlines,
                }
            })
            .collect(),
        ..Default::default()
    }
}

fn paragraph_format(list: bool) -> ParagraphFormat {
    ParagraphFormat {
        left_indent: Some(if list { LIST_INDENT } else { 0.0 }),
        text_indent: Some(if list { -18.0 } else { 0.0 }),
        ..Default::default()
    }
}

pub(super) fn format(run: &NoteRun) -> CharacterFormat {
    let mut decorations = Vec::new();
    if run.underline {
        decorations.push(TextDecorations::underline().get(0));
    }
    if run.strike {
        decorations.push(TextDecorations::strikethrough().get(0));
    }
    CharacterFormat {
        font_weight: Some(if run.bold {
            FontWeight::Bold
        } else {
            FontWeight::Normal
        }),
        font_style: Some(if run.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        }),
        text_decorations: Some(
            (!decorations.is_empty()).then(|| TextDecorationCollection::from_items(decorations)),
        ),
        ..Default::default()
    }
}

fn hyperlink(inlines: Vec<InlineSnapshot>, uri: &str) -> InlineSnapshot {
    InlineSnapshot {
        format: CharacterFormat::default(),
        kind: InlineSnapshotKind::Hyperlink {
            inlines,
            field_id: None,
            behavior: Default::default(),
            navigate_uri: Some(uri.into()),
            target_name: None,
            is_enabled: true,
            is_tab_stop: true,
            tab_index: 0,
            is_visited: false,
            underline_style: UnderlineStyle::Single,
        },
    }
}

pub(crate) fn read(field: &RichEditBox) -> RichNote {
    from_snapshot(&field.rich_document().snapshot())
}

/// A host save/close has the same composition boundary as losing native focus.
/// Publish the accepted preedit once, then reset the IME so its late commit
/// cannot insert that text a second time when the surface is resumed.
pub(crate) fn settle_composition(field: &RichEditBox) -> Result<(), EditError> {
    let editor = field.editor();
    if editor.is_composing() {
        let text = editor
            .active_composition_range()
            .map(|range| editor.rich_document().get_text(range))
            .transpose()?
            .unwrap_or_default();
        field.commit_composition(&text)?;
        field.text_input_method_client().request_reset();
    }
    Ok(())
}

pub(super) fn strike_state(field: &RichEditBox) -> bool {
    let editor = field.editor();
    if editor.selection().is_empty() {
        return editor
            .current_character_format()
            .ok()
            .and_then(|format| format.text_decorations.flatten())
            .is_some_and(|items| {
                items
                    .list()
                    .snapshot()
                    .iter()
                    .any(|item| item.location() == TextDecorationLocation::Strikethrough)
            });
    }
    editor.selected_fragment().ok().is_some_and(|fragment| {
        from_snapshot(&fragment)
            .paragraphs
            .iter()
            .flat_map(|p| &p.runs)
            .all(|run| run.strike)
    })
}

pub(super) fn selected_link(field: &RichEditBox) -> Option<String> {
    if field.editor().selection().is_empty() {
        return link_at_caret(field).map(|(_, url)| url);
    }
    let note = from_snapshot(&field.editor().selected_fragment().ok()?);
    let mut runs = note.paragraphs.iter().flat_map(|p| &p.runs);
    let first = runs.next()?.link.as_ref()?;
    runs.all(|run| run.link.as_ref() == Some(first))
        .then(|| first.clone())
}

fn link_at_caret(field: &RichEditBox) -> Option<(std::ops::Range<usize>, String)> {
    let caret = field.editor().selection().active;
    fn visit(
        items: &[InlineSnapshot],
        offset: &mut usize,
        caret: usize,
    ) -> Option<(std::ops::Range<usize>, String)> {
        for item in items {
            let end = *offset + item.text_length();
            if let InlineSnapshotKind::Hyperlink { navigate_uri, .. } = &item.kind {
                if *offset <= caret && caret <= end {
                    if let Some(url) = navigate_uri.as_deref().and_then(safe_link) {
                        return Some((*offset..end, url));
                    }
                }
            }
            if let Some(children) = item.children() {
                if let Some(found) = visit(children, offset, caret) {
                    return Some(found);
                }
            }
            *offset = end;
        }
        None
    }
    let mut offset = 0;
    for paragraph in &field.rich_document().snapshot().paragraphs {
        if let Some(found) = visit(&paragraph.inlines, &mut offset, caret) {
            return Some(found);
        }
        offset += 1;
    }
    None
}

#[cfg(test)]
#[path = "notes_document_tests.rs"]
mod tests;

fn from_snapshot(snapshot: &DocumentSnapshot) -> RichNote {
    RichNote {
        paragraphs: snapshot
            .paragraphs
            .iter()
            .map(|p| {
                let mut paragraph = NoteParagraph::default();
                flatten(&p.inlines, &p.format.character, None, &mut paragraph);
                if let Some((list, length)) = list_prefix(p) {
                    paragraph.list = Some(list);
                    let mut left = length;
                    for run in &mut paragraph.runs {
                        let bytes = run
                            .text
                            .char_indices()
                            .nth(left)
                            .map_or(run.text.len(), |(byte, _)| byte);
                        let count = run.text[..bytes].chars().count();
                        run.text.drain(..bytes);
                        left -= count;
                        if left == 0 {
                            break;
                        }
                    }
                    paragraph.runs.retain(|run| !run.text.is_empty());
                }
                paragraph
            })
            .collect(),
    }
}

fn flatten(
    inlines: &[InlineSnapshot],
    inherited: &CharacterFormat,
    link: Option<&str>,
    paragraph: &mut NoteParagraph,
) {
    for inline in inlines {
        let mut resolved = inherited.overlay(&inline.format);
        match &inline.kind {
            InlineSnapshotKind::Bold(_) => resolved.font_weight = Some(FontWeight::Bold),
            InlineSnapshotKind::Italic(_) => resolved.font_style = Some(FontStyle::Italic),
            _ => {}
        }
        let mut run = NoteRun {
            bold: resolved
                .font_weight
                .is_some_and(|v| v >= FontWeight::SemiBold),
            italic: resolved.font_style == Some(FontStyle::Italic),
            link: link.and_then(safe_link),
            ..Default::default()
        };
        if let Some(Some(items)) = &resolved.text_decorations {
            run.underline = items
                .list()
                .snapshot()
                .iter()
                .any(|item| item.location() == TextDecorationLocation::Underline);
            run.strike = items
                .list()
                .snapshot()
                .iter()
                .any(|item| item.location() == TextDecorationLocation::Strikethrough);
        }
        match &inline.kind {
            InlineSnapshotKind::Run(text) => {
                run.text = text.clone();
                paragraph.push(run);
            }
            InlineSnapshotKind::LineBreak => {
                run.text = "\n".into();
                paragraph.push(run);
            }
            InlineSnapshotKind::Hyperlink {
                inlines,
                navigate_uri,
                ..
            } => flatten(inlines, &resolved, navigate_uri.as_deref(), paragraph),
            InlineSnapshotKind::Underline(children) => {
                let mut items = resolved
                    .text_decorations
                    .clone()
                    .flatten()
                    .map_or(Vec::new(), |d| d.list().snapshot().to_vec());
                items.push(TextDecorations::underline().get(0));
                resolved.text_decorations = Some(Some(TextDecorationCollection::from_items(items)));
                flatten(children, &resolved, link, paragraph);
            }
            _ => {
                if let Some(children) = inline.children() {
                    flatten(children, &resolved, link, paragraph);
                }
            }
        }
    }
}

fn list_prefix(paragraph: &ParagraphSnapshot) -> Option<(u32, usize)> {
    if paragraph.format.left_indent != Some(LIST_INDENT)
        || paragraph.format.text_indent != Some(-18.0)
    {
        return None;
    }
    let text = paragraph.text();
    if text.starts_with("• ") {
        return Some((0, 2));
    }
    let (number, _) = text.split_once(". ")?;
    let value: u32 = number.parse().ok()?;
    (value > 0).then_some((value, number.len() + 2))
}

pub(super) fn list(field: &RichEditBox, numbered: bool) -> Result<(), EditError> {
    let editor = field.editor();
    let selection = editor.selection();
    let range = selection.range();
    let snapshot = field.rich_document().snapshot();
    let mut offset = 0;
    let mut paragraphs = Vec::new();
    for p in &snapshot.paragraphs {
        let end = offset + p.text_length();
        if offset <= range.end && end >= range.start && (range.is_empty() || offset < range.end) {
            paragraphs.push((offset, list_prefix(p)));
        }
        offset = end + 1;
    }
    let disable = !paragraphs.is_empty()
        && paragraphs
            .iter()
            .all(|(_, prefix)| prefix.is_some_and(|(n, _)| (n > 0) == numbered));
    let changes = paragraphs
        .iter()
        .enumerate()
        .map(|(index, (start, old))| {
            (
                *start,
                old.map_or(0, |(_, n)| n),
                if disable {
                    String::new()
                } else if numbered {
                    format!("{}. ", index + 1)
                } else {
                    "• ".into()
                },
            )
        })
        .collect::<Vec<_>>();
    editor.edit_group(|editor| {
        for (start, old, prefix) in changes.iter().rev() {
            editor.set_selection(EditorSelection::new(*start, start + old))?;
            if prefix.is_empty() {
                editor.delete_selection()?;
            } else {
                editor.insert_text(prefix, EditKind::Formatting)?;
            }
            editor.apply_paragraph_format(&paragraph_format(!disable))?;
        }
        let remap = |value: usize| {
            let mut adjusted = value as isize;
            for (start, old, prefix) in &changes {
                if value >= *start {
                    adjusted +=
                        prefix.encode_utf16().count() as isize - (*old).min(value - start) as isize;
                }
            }
            adjusted.max(0) as usize
        };
        editor.set_selection(EditorSelection::new(
            remap(selection.anchor),
            remap(selection.active),
        ))
    })
}

/// Enter continues a formatted list; Enter in an empty item exits it. The entire
/// operation is one native undo transaction and never intercepts IME preedit.
pub(super) fn list_enter(field: &RichEditBox) -> Result<bool, EditError> {
    let editor = field.editor();
    let selection = editor.selection();
    if !selection.is_empty() || editor.is_composing() {
        return Ok(false);
    }
    let snapshot = field.rich_document().snapshot();
    let mut offset = 0;
    for p in &snapshot.paragraphs {
        let end = offset + p.text_length();
        if selection.active <= end {
            let Some((number, prefix)) = list_prefix(p) else {
                return Ok(false);
            };
            editor.edit_group(|editor| {
                if p.text_length() == prefix {
                    editor.set_selection(EditorSelection::new(offset, end))?;
                    editor.delete_selection()?;
                    editor.apply_paragraph_format(&paragraph_format(false))?;
                } else {
                    let prefix = if number == 0 {
                        "• ".into()
                    } else {
                        format!("{}. ", number.saturating_add(1))
                    };
                    editor.insert_text(&format!("\n{prefix}"), EditKind::Typing)?;
                    editor.apply_paragraph_format(&paragraph_format(true))?;
                }
                Ok(())
            })?;
            return Ok(true);
        }
        offset = end + 1;
    }
    Ok(false)
}

pub(super) fn strike(field: &RichEditBox) -> Result<(), EditError> {
    let editor = field.editor();
    let selection = editor.selection();
    if selection.is_empty() {
        let current = editor.current_character_format()?;
        let items = current
            .text_decorations
            .flatten()
            .map_or(Vec::new(), |d| d.list().snapshot().to_vec());
        let enabled = items
            .iter()
            .any(|d| d.location() == TextDecorationLocation::Strikethrough);
        let mut items = items
            .into_iter()
            .filter(|d| d.location() != TextDecorationLocation::Strikethrough)
            .collect::<Vec<_>>();
        if !enabled {
            items.push(TextDecorations::strikethrough().get(0));
        }
        return field.apply_format(&CharacterFormat {
            text_decorations: Some(Some(TextDecorationCollection::from_items(items))),
            ..Default::default()
        });
    }
    let fragment = editor.selected_fragment()?;
    let mut note = from_snapshot(&fragment);
    let enabled = note
        .paragraphs
        .iter()
        .flat_map(|p| &p.runs)
        .all(|r| r.strike);
    for run in note.paragraphs.iter_mut().flat_map(|p| &mut p.runs) {
        run.strike = !enabled;
    }
    let mut replacement = snapshot(&note);
    // A partial inline selection must not reset its enclosing paragraph style.
    for (new, old) in replacement.paragraphs.iter_mut().zip(&fragment.paragraphs) {
        new.format = old.format.clone();
    }
    editor.edit_group(|editor| {
        field.replace_selection(&replacement)?;
        editor.set_selection(selection)
    })
}

pub(super) fn link(field: &RichEditBox, uri: Option<&str>) -> Result<(), EditError> {
    let editor = field.editor();
    let selection = editor.selection();
    let target = if selection.is_empty() {
        link_at_caret(field)
            .map(|(range, _)| range)
            .unwrap_or_else(|| selection.range())
    } else {
        selection.range()
    };
    let mut fragment = field.rich_document().slice(target.clone())?;
    // Work in the same native history/selection engine as typing and paste.
    for paragraph in &mut fragment.paragraphs {
        fn unwrap(items: &mut Vec<InlineSnapshot>) {
            let old = std::mem::take(items);
            for mut item in old {
                if let InlineSnapshotKind::Hyperlink { mut inlines, .. } = item.kind {
                    unwrap(&mut inlines);
                    items.extend(inlines);
                } else {
                    if let InlineSnapshotKind::Span(children)
                    | InlineSnapshotKind::Bold(children)
                    | InlineSnapshotKind::Italic(children)
                    | InlineSnapshotKind::Underline(children) = &mut item.kind
                    {
                        unwrap(children);
                    }
                    items.push(item);
                }
            }
        }
        unwrap(&mut paragraph.inlines);
        if let Some(uri) = uri.and_then(safe_link) {
            paragraph.inlines = vec![hyperlink(std::mem::take(&mut paragraph.inlines), &uri)];
        }
    }
    fragment.link_fields.clear();
    if target.is_empty() {
        if let Some(uri) = uri.and_then(safe_link) {
            fragment = snapshot(&RichNote {
                paragraphs: vec![NoteParagraph {
                    runs: vec![NoteRun {
                        text: uri.clone(),
                        link: Some(uri),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
            });
        } else {
            return Ok(());
        }
    }
    let length = fragment.text_length();
    editor.edit_group(|editor| {
        editor.set_selection(EditorSelection::new(target.start, target.end))?;
        editor.insert_fragment(&fragment, EditKind::Formatting)?;
        editor.set_selection(if target.is_empty() {
            EditorSelection::caret(selection.active + length)
        } else {
            selection
        })
    })
}
