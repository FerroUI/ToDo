use ferroui_base::input::{FocusManager, PointerEventArgs};
use ferroui_base::media::{Color, SolidColorBrush, TextDecorations, TextWrapping};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::documents::{Hyperlink, Paragraph, Run, UnderlineStyle};
use ferroui_controls::{Control, ListBox, RichTextBlock};
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_domain::{hashtags, View};
use todo_state::{AppViewModel, Panel};

/// Keep ordinary rows on TextBlock's small path. A title with hashtags uses the
/// native rich-text host so links retain inline wrapping and real automation.
pub(super) fn view(
    vm: Rc<AppViewModel>,
    title: String,
    completed: bool,
    selection_mode: bool,
    foreground: &str,
    accent: &str,
) -> Element {
    if selection_mode || hashtags::ranges(&title).next().is_none() {
        let text = text_block()
            .text(title)
            .font_size(14.0)
            .foreground(foreground)
            .text_wrapping(TextWrapping::Wrap)
            .text_trimming(<dyn ferroui_base::media::TextTrimming>::word_ellipsis())
            .max_lines(2);
        return if completed {
            text.text_decorations(TextDecorations::strikethrough())
                .into()
        } else {
            text.into()
        };
    }
    let accent = accent.to_owned();
    rich_text_block()
        // Structured content is installed on mount. A changed title or style
        // must not retain the previous item's spans in a recycled row.
        .key(format!("title:{completed}:{accent}:{title}"))
        .name("HashtagTaskTitle")
        .automation_properties_name(title.clone())
        .font_size(14.0)
        .foreground(foreground)
        .is_text_selection_enabled(false)
        .focusable(true)
        .text_wrapping(TextWrapping::Wrap)
        .text_trimming(<dyn ferroui_base::media::TextTrimming>::word_ellipsis())
        .max_lines(2)
        .with(move |block| {
            if completed {
                block.set_text_decorations(Some(TextDecorations::strikethrough()));
            }
            let paragraph = Paragraph::new();
            let mut end = 0;
            let mut subscriptions = Vec::new();
            for range in hashtags::ranges(&title) {
                if end < range.start {
                    paragraph.add_child(Run::with_text(Some(&title[end..range.start])));
                }
                let query = title[range.clone()].to_owned();
                let link = Hyperlink::with_text(&query, None);
                link.set_underline_style(UnderlineStyle::None);
                if let Ok(color) = Color::parse(&accent) {
                    link.set_foreground(Some(SolidColorBrush::with_color(color).into()));
                }
                if completed {
                    link.set_text_decorations(Some(TextDecorations::strikethrough()));
                }
                AutomationProperties::set_name(&link, Some(&format!("Search for {query}")));
                AutomationProperties::set_help_text(
                    &link,
                    Some("Find this tag in tasks, notes, steps and categories."),
                );
                let vm = Rc::downgrade(&vm);
                let source = block.downgrade();
                subscriptions.push(link.click(move |event| {
                    event.set_handled(true);
                    let Some(vm) = vm.upgrade() else { return };
                    if vm.selection_mode() {
                        return;
                    }
                    let list = source.upgrade().and_then(|source| {
                        source
                            .get_self_and_visual_ancestors()
                            .find_map(|visual| visual.cast::<ListBox>())
                            .map(|list| list.downgrade())
                    });
                    vm.navigate(View::Search(query.clone()));
                    // The activated title can leave the realized viewport.
                    // ListBox is not itself focusable: focus a real result,
                    // after layout, unless a later action changed the route.
                    let vm = Rc::downgrade(&vm);
                    let query = query.clone();
                    Dispatcher::ui_thread().post_local(
                        move || {
                            let Some(vm) = vm.upgrade().filter(|vm| {
                                vm.view() == View::Search(query.clone())
                                    && vm.panel() == Panel::None
                                    && vm.selected().is_none()
                            }) else {
                                return;
                            };
                            if let Some(list) = list
                                .and_then(|list| list.upgrade())
                                .filter(|list| list.is_attached_to_visual_tree())
                            {
                                if !list.is_measure_valid() || !list.is_arrange_valid() {
                                    list.update_layout();
                                }
                                if let Some(item) =
                                    list.get_realized_containers().into_iter().find(|item| {
                                        FocusManager::can_focus(item)
                                            && item.data_context().is_some_and(|item| {
                                                matches!(
                                                    item.downcast_ref::<crate::rows::Row>(),
                                                    Some(crate::rows::Row::Task { .. })
                                                )
                                            })
                                    })
                                {
                                    list.set_selected_index(list.index_from_container(&item));
                                    item.focus();
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
            if end < title.len() {
                paragraph.add_child(Run::with_text(Some(&title[end..])));
            }
            block.blocks().add(paragraph);
            on_cleanup(move || drop(subscriptions));
        })
        .into()
}

/// A native inline link owns its press; task drag/swipe must not also arm.
pub(crate) fn is_link_press(event: &PointerEventArgs) -> bool {
    event
        .source()
        .and_then(|source| source.cast::<Control>())
        .is_some_and(|source| {
            source
                .get_self_and_visual_ancestors()
                .filter_map(|visual| visual.cast::<RichTextBlock>())
                .any(|block| {
                    block.name().as_deref() == Some("HashtagTaskTitle")
                        && block
                            .hyperlink_at(event.get_position(Some(&block)))
                            .is_some()
                })
        })
}
