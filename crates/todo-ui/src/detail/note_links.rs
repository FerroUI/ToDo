use super::notes_document;
use ferroui_base::input::FocusManager;
use ferroui_base::media::{Color, SolidColorBrush, TextWrapping};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::documents::{
    Hyperlink, InlineCollection, LineBreak, Paragraph, Run, UnderlineStyle,
};
use ferroui_controls::{ListBox, TopLevel};
use ferroui_view::prelude::*;
use std::ops::Range;
use std::rc::Rc;
use todo_domain::{
    hashtags,
    notes::{safe_link, NoteParagraph, NoteRun, RichNote},
    View,
};
use todo_state::{AppViewModel, HostRequest, Panel};

pub(super) fn has_links(text: &str) -> bool {
    !ranges(text).is_empty()
}

fn ranges(text: &str) -> Vec<(Range<usize>, bool)> {
    let mut links = Vec::new();
    let mut offset = 0;
    for token_with_space in text.split_inclusive(char::is_whitespace) {
        let token = token_with_space.trim_end_matches(char::is_whitespace);
        let start = token.char_indices().map(|(i, _)| i).find(|i| {
            token[*i..]
                .get(..8)
                .is_some_and(|s| s.eq_ignore_ascii_case("https://"))
                || token[*i..]
                    .get(..7)
                    .is_some_and(|s| s.eq_ignore_ascii_case("http://"))
        });
        if let Some(start) = start {
            let mut url = token[start..].trim_end_matches(['.', ',', ';', ':', '!', '?']);
            while let Some(last) = url.chars().last().filter(|c| matches!(c, ')' | ']' | '}')) {
                let open = match last {
                    ')' => '(',
                    ']' => '[',
                    _ => '{',
                };
                if url.chars().filter(|c| *c == last).count()
                    <= url.chars().filter(|c| *c == open).count()
                {
                    break;
                }
                url = &url[..url.len() - last.len_utf8()];
            }
            if safe_link(url).is_some() {
                links.push((offset + start..offset + start + url.len(), false));
            }
        }
        offset += token_with_space.len();
    }
    for range in hashtags::ranges(text) {
        if !links
            .iter()
            .any(|(url, _)| range.start < url.end && range.end > url.start)
        {
            links.push((range, true));
        }
    }
    links.sort_by_key(|(range, _)| range.start);
    links
}

pub(super) fn view(
    vm: Rc<AppViewModel>,
    note: RichNote,
    name: &str,
    foreground: &str,
    accent: &str,
) -> Element {
    let accent = accent.to_owned();
    rich_text_block()
        .key(format!("{name}:{accent}:{}", note.to_html()))
        .name(name)
        .automation_properties_name(note.text())
        .font_size(14.0)
        .foreground(foreground)
        .is_text_selection_enabled(true)
        .focusable(true)
        .text_wrapping(TextWrapping::Wrap)
        .margin((6.0, 8.0))
        .with(move |block| {
            let mut subscriptions = Vec::new();
            let source = block.downgrade();
            for item in &note.paragraphs {
                let paragraph = Paragraph::new();
                if item.list.is_some() {
                    paragraph.add_child(Run::with_text(Some(&item.prefix())));
                    paragraph.set_left_indent(24.0);
                    paragraph.set_text_indent(-18.0);
                }
                let text = item
                    .runs
                    .iter()
                    .map(|run| run.text.as_str())
                    .collect::<String>();
                let segments = segments(item);
                let mut end = 0;
                for (range, tag, target) in segments {
                    if end < range.start {
                        append_runs(&paragraph.inlines(), item, end..range.start);
                    }
                    let caption = &text[range.clone()];
                    let link = Hyperlink::new();
                    append_runs(&link.inlines(), item, range.clone());
                    link.set_underline_style(UnderlineStyle::Single);
                    if let Ok(color) = Color::parse(&accent) {
                        link.set_foreground(Some(SolidColorBrush::with_color(color).into()));
                    }
                    AutomationProperties::set_name(
                        &link,
                        Some(&if tag {
                            format!("Search for {caption}")
                        } else {
                            format!("Open {target}")
                        }),
                    );
                    let vm = Rc::downgrade(&vm);
                    let source = source.clone();
                    subscriptions.push(link.click(move |event| {
                        event.set_handled(true);
                        let Some(vm) = vm.upgrade() else { return };
                        if !tag {
                            if let Some(url) = safe_link(&target) {
                                vm.request(HostRequest::OpenUrl { url });
                            }
                            return;
                        }
                        let root = source
                            .upgrade()
                            .and_then(|source| TopLevel::get_top_level(Some(&source)))
                            .map(|root| root.downgrade());
                        vm.navigate(View::Search(target.clone()));
                        let query = target.clone();
                        let vm = Rc::downgrade(&vm);
                        Dispatcher::ui_thread().post_local(
                            move || {
                                let Some(vm) = vm.upgrade().filter(|vm| {
                                    vm.view() == View::Search(query.clone())
                                        && vm.panel() == Panel::None
                                        && vm.selected().is_none()
                                }) else {
                                    return;
                                };
                                if let Some(root) = root.and_then(|root| root.upgrade()) {
                                    root.update_layout();
                                    if let Some(list) = root
                                        .find_descendant_of_type_where::<ListBox>(false, |list| {
                                            list.name().as_deref() == Some("TaskList")
                                        })
                                    {
                                        if let Some(item) = list
                                            .get_realized_containers()
                                            .into_iter()
                                            .find(|item| {
                                                FocusManager::can_focus(item)
                                                    && item.data_context().is_some_and(|data| {
                                                        matches!(
                                                            data.downcast_ref::<crate::rows::Row>(),
                                                            Some(crate::rows::Row::Task { .. })
                                                        )
                                                    })
                                            })
                                        {
                                            list.set_selected_index(
                                                list.index_from_container(&item),
                                            );
                                            item.focus();
                                        }
                                    }
                                }
                                drop(vm);
                            },
                            DispatcherPriority::BACKGROUND,
                        );
                    }));
                    paragraph.add_child(link);
                    end = range.end;
                }
                if end < text.len() {
                    append_runs(&paragraph.inlines(), item, end..text.len());
                }
                block.blocks().add(paragraph);
            }
            on_cleanup(move || drop(subscriptions));
        })
        .into()
}

/// Build token ranges across formatting boundaries, so a bold prefix does not
/// turn #Release into two different searches. Explicit links own their text.
fn segments(paragraph: &NoteParagraph) -> Vec<(Range<usize>, bool, String)> {
    let text = paragraph
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();
    let mut explicit: Vec<(Range<usize>, bool, String)> = Vec::new();
    let mut offset = 0;
    for run in &paragraph.runs {
        let end = offset + run.text.len();
        if let Some(url) = run.link.as_deref().and_then(safe_link) {
            if let Some(last) = explicit
                .last_mut()
                .filter(|last| last.0.end == offset && last.2 == url)
            {
                last.0.end = end;
            } else {
                explicit.push((offset..end, false, url));
            }
        }
        offset = end;
    }
    let mut links = ranges(&text)
        .into_iter()
        .filter(|(range, _)| {
            !explicit
                .iter()
                .any(|(owned, _, _)| range.start < owned.end && range.end > owned.start)
        })
        .map(|(range, tag)| {
            let target = text[range.clone()].to_owned();
            (range, tag, target)
        })
        .collect::<Vec<_>>();
    links.extend(explicit);
    links.sort_by_key(|link| link.0.start);
    links
}

fn append_runs(inlines: &InlineCollection, paragraph: &NoteParagraph, range: Range<usize>) {
    let mut offset = 0;
    for run in &paragraph.runs {
        let end = offset + run.text.len();
        let start = range.start.max(offset);
        let stop = range.end.min(end);
        if start < stop {
            append_text(inlines, &run.text[start - offset..stop - offset], run);
        }
        offset = end;
    }
}

fn append_text(inlines: &InlineCollection, text: &str, format: &NoteRun) {
    for (index, text) in text.split('\n').enumerate() {
        if index != 0 {
            inlines.add(LineBreak::new());
        }
        if !text.is_empty() {
            let run = Run::with_text(Some(text));
            let _ = notes_document::format(format).try_apply_to(&run);
            inlines.add(run);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_span_styles_and_urls_own_fragments() {
        let note = RichNote::from_html("<p><b>#Rel</b>ease https://example.com/page#fragment, <u>HTTPS://example.com/a(b)</u></p>").unwrap();
        let links = segments(&note.paragraphs[0]);
        assert_eq!(
            links
                .iter()
                .map(|(_, tag, target)| (*tag, target.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (true, "#Release"),
                (false, "https://example.com/page#fragment"),
                (false, "HTTPS://example.com/a(b)")
            ]
        );
        let note =
            RichNote::from_html("<a href='https://example.com/'><b>#Rel</b>ease</a>").unwrap();
        let links = segments(&note.paragraphs[0]);
        assert_eq!(links.len(), 1);
        assert!(!links[0].1);
    }
}
