//! Portable task representations shared by the clipboard and native drag-and-drop.
//!
//! A drop transfer is borrowed only for the synchronous drop callback. Clipboard
//! ownership and disposal remain with the caller across asynchronous reads. Neither
//! path authorizes a cut: only the view model may recognize a local clipboard offer.
use ferroui_base::input::{
    DataFormat, DataFormatOf, DataTransferItem, IAsyncDataTransfer, IDataTransfer,
};
use std::any::Any;
use std::rc::Rc;
use todo_state::{TaskClipboard, MAX_TASK_CLIPBOARD_BYTES};

const MAX_HTML_BYTES: usize = 8 * 1024 * 1024;
const MAX_CF_HEADER_BYTES: usize = 4096;
const MAX_TEXT_ITEMS: usize = 1000;
const HTML_MARKER: &str = "data-ferroui-todo-v1=\"";
const INVALID_TASKS: &str = "The transferred task data is invalid";
const INVALID_HTML: &str = "The transferred task HTML is invalid";

fn task_format() -> DataFormatOf<Rc<[u8]>> {
    DataFormat::create_bytes_application_format("FerroUI.ToDo.Tasks")
}

fn html_format(identifier: &str) -> DataFormatOf<String> {
    DataFormat::create_string_platform_format(identifier)
}

fn windows_html_format() -> DataFormatOf<Rc<[u8]>> {
    // CF_HTML is UTF-8, whereas Win32 serializes a String format as UTF-16.
    DataFormat::create_bytes_platform_format("HTML Format")
}

/// Creates one portable offer with plain text, task bytes and native standard HTML.
pub(crate) fn item(data: &TaskClipboard) -> Result<Rc<DataTransferItem>, String> {
    let json = data.to_json()?;
    let text = data.text();
    if text.len() > MAX_TASK_CLIPBOARD_BYTES {
        return Err("Task text exceeds 4 MiB".into());
    }
    let html = data.to_html()?;
    if html.len() > MAX_HTML_BYTES {
        return Err("Task HTML exceeds 8 MiB".into());
    }
    let item = DataTransferItem::new();
    item.set(&DataFormat::text(), Some(text));
    item.set(&task_format(), Some(Rc::<[u8]>::from(json.into_bytes())));
    #[cfg(target_os = "windows")]
    item.set(
        &windows_html_format(),
        Some(Rc::<[u8]>::from(encode_cf_html(&html)?)),
    );
    #[cfg(target_os = "macos")]
    item.set(&html_format("public.html"), Some(html));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    item.set(&html_format("text/html"), Some(html));
    Ok(item)
}

#[derive(Clone, Copy)]
enum Representation {
    Tasks,
    Html,
    WindowsHtml,
    Text,
}

fn formats() -> [(Representation, DataFormat); 5] {
    use Representation::*;
    // Native HTML first; the other recognized aliases remain readable in portable
    // transfers and in bridges that expose another platform's standard format.
    let html = (Html, html_format("text/html").into());
    let mac = (Html, html_format("public.html").into());
    let win = (WindowsHtml, windows_html_format().into());
    let html = if cfg!(target_os = "windows") {
        [win, html, mac]
    } else if cfg!(target_os = "macos") {
        [mac, html, win]
    } else {
        [html, mac, win]
    };
    let [first, second, third] = html;
    [
        (Tasks, task_format().into()),
        first,
        second,
        third,
        (Text, DataFormat::text().into()),
    ]
}

/// Only format metadata is inspected during hover; delayed payloads stay unread.
pub(crate) fn has_supported(data: &dyn IDataTransfer) -> bool {
    // A failing native format provider must decline the hover, without requesting
    // payloads or changing the transfer's lifetime.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let offered = data.formats();
        formats().iter().any(|(_, wanted)| offered.contains(wanted))
    }))
    .unwrap_or(false)
}

/// Materializes transferable values before the borrowed native drop lease ends.
pub(crate) fn read_drop(
    data: &dyn IDataTransfer,
) -> Result<(Option<TaskClipboard>, String), String> {
    // Synchronous IDataTransfer has no platform-error channel. Native backends
    // throw failures as panics, unlike their async clipboard counterparts. This
    // boundary contains only borrowed reads; it cannot partially mutate the app.
    // Recovery requires an unwind-enabled native build (the dist-unwind profile).
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| read_drop_inner(data)))
        .unwrap_or_else(|_| Err("The dropped data could not be read. Please try again.".into()))
}

fn read_drop_inner(data: &dyn IDataTransfer) -> Result<(Option<TaskClipboard>, String), String> {
    let items = data.items();
    let mut result = Reader::default();
    for (representation, format) in formats() {
        for item in items.iter() {
            if item.formats().contains(&format) {
                if let Some(tasks) = result.accept(representation, item.try_get_raw(&format))? {
                    let text = tasks.text();
                    return Ok((Some(tasks), text));
                }
            }
        }
    }
    Ok((None, result.text))
}

/// The caller retains and disposes its retrieved clipboard transfer, including
/// when this future fails or is canceled. Stronger representations win globally.
pub(crate) async fn read_clipboard(
    data: &dyn IAsyncDataTransfer,
) -> Result<(Option<TaskClipboard>, String), String> {
    let items = data.try_items().map_err(|error| error.to_string())?;
    let mut result = Reader::default();
    for (representation, format) in formats() {
        for item in items.iter() {
            if item
                .try_formats()
                .map_err(|error| error.to_string())?
                .contains(&format)
            {
                let raw = item
                    .try_get_raw_async(&format)
                    .await
                    .map_err(|error| error.to_string())?;
                if let Some(tasks) = result.accept(representation, raw)? {
                    let text = tasks.text();
                    return Ok((Some(tasks), text));
                }
            }
        }
    }
    Ok((None, result.text))
}

#[derive(Default)]
struct Reader {
    text: String,
    text_items: usize,
    html_items: usize,
}

impl Reader {
    fn accept(
        &mut self,
        representation: Representation,
        raw: Option<Rc<dyn Any>>,
    ) -> Result<Option<TaskClipboard>, String> {
        let raw = raw.ok_or("The advertised transfer data could not be read")?;
        match representation {
            Representation::Tasks => {
                let bytes = raw.downcast_ref::<Rc<[u8]>>().ok_or(INVALID_TASKS)?;
                if bytes.len() > MAX_TASK_CLIPBOARD_BYTES {
                    return Err("Task clipboard exceeds 4 MiB".into());
                }
                let json = std::str::from_utf8(bytes).map_err(|_| INVALID_TASKS)?;
                TaskClipboard::from_json(json).map(Some)
            }
            Representation::Html | Representation::WindowsHtml => {
                self.html_items += 1;
                if self.html_items > MAX_TEXT_ITEMS {
                    return Err("Paste at most 1,000 HTML items at a time".into());
                }
                if matches!(representation, Representation::WindowsHtml) {
                    let bytes = raw.downcast_ref::<Rc<[u8]>>().ok_or(INVALID_HTML)?;
                    if bytes.len() > MAX_HTML_BYTES + MAX_CF_HEADER_BYTES {
                        return Err("Task HTML exceeds 8 MiB".into());
                    }
                    let html = std::str::from_utf8(bytes).map_err(|_| INVALID_HTML)?;
                    // Ordinary external HTML is never interpreted as tasks. A
                    // marked payload must additionally have valid CF_HTML offsets.
                    if !html.contains(HTML_MARKER) {
                        return Ok(None);
                    }
                    TaskClipboard::from_html(cf_html_fragment(bytes)?)
                } else {
                    let html = raw.downcast_ref::<String>().ok_or(INVALID_HTML)?;
                    if html.len() > MAX_HTML_BYTES {
                        return Err("Task HTML exceeds 8 MiB".into());
                    }
                    TaskClipboard::from_html(html)
                }
            }
            Representation::Text => {
                self.text_items += 1;
                if self.text_items > MAX_TEXT_ITEMS {
                    return Err("Paste at most 1,000 text items at a time".into());
                }
                let text = raw
                    .downcast_ref::<String>()
                    .ok_or("The transferred text is invalid")?;
                let separator = usize::from(!self.text.is_empty() && !text.is_empty());
                if self
                    .text
                    .len()
                    .checked_add(separator)
                    .and_then(|size| size.checked_add(text.len()))
                    .is_none_or(|size| size > MAX_TASK_CLIPBOARD_BYTES)
                {
                    return Err("Task text exceeds 4 MiB".into());
                }
                if separator != 0 {
                    self.text.push('\n');
                }
                self.text.push_str(text);
                Ok(None)
            }
        }
    }
}

/// https://learn.microsoft.com/en-us/windows/win32/dataxchg/html-clipboard-format
/// All offsets are UTF-8 byte offsets from the beginning of the complete buffer.
#[cfg(any(target_os = "windows", test))]
fn encode_cf_html(fragment: &str) -> Result<Vec<u8>, String> {
    const PREFIX: &str = "<html><head><meta charset=\"utf-8\"></head><body><!--StartFragment-->";
    const SUFFIX: &str = "<!--EndFragment--></body></html>";
    if fragment.len() > MAX_HTML_BYTES - PREFIX.len() - SUFFIX.len() {
        return Err("Task HTML exceeds 8 MiB".into());
    }
    let header = |start, end, fragment_start, fragment_end| {
        format!(
            "Version:1.0\r\nStartHTML:{start:010}\r\nEndHTML:{end:010}\r\nStartFragment:{fragment_start:010}\r\nEndFragment:{fragment_end:010}\r\n"
        )
    };
    let start = header(0, 0, 0, 0).len();
    let fragment_start = start + PREFIX.len();
    let fragment_end = fragment_start + fragment.len();
    let end = fragment_end + SUFFIX.len();
    let mut bytes = header(start, end, fragment_start, fragment_end).into_bytes();
    bytes.extend_from_slice(PREFIX.as_bytes());
    bytes.extend_from_slice(fragment.as_bytes());
    bytes.extend_from_slice(SUFFIX.as_bytes());
    // The terminator is not part of the HTML or fragment byte ranges.
    bytes.push(0);
    Ok(bytes)
}

fn cf_html_fragment(bytes: &[u8]) -> Result<&str, String> {
    if bytes.len() > MAX_HTML_BYTES + MAX_CF_HEADER_BYTES {
        return Err("Task HTML exceeds 8 MiB".into());
    }
    if !bytes.starts_with(b"Version:") {
        return Err(INVALID_HTML.into());
    }
    let mut offsets = [None; 4];
    let mut cursor = 0;
    while cursor < bytes.len() {
        if offsets[0].is_some_and(|start| start >= 0 && cursor >= start as usize)
            || bytes[cursor] == b'<'
        {
            break;
        }
        if cursor >= MAX_CF_HEADER_BYTES {
            return Err(INVALID_HTML.into());
        }
        let remaining = &bytes[cursor..bytes.len().min(MAX_CF_HEADER_BYTES)];
        let end = remaining
            .iter()
            .position(|byte| matches!(byte, b'\r' | b'\n'))
            .ok_or(INVALID_HTML)?;
        let line = std::str::from_utf8(&remaining[..end]).map_err(|_| INVALID_HTML)?;
        let (key, value) = line.split_once(':').ok_or(INVALID_HTML)?;
        if !key.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return Err(INVALID_HTML.into());
        }
        if let Some(index) = ["StartHTML", "EndHTML", "StartFragment", "EndFragment"]
            .iter()
            .position(|candidate| *candidate == key)
        {
            let value = value.trim().parse::<i64>().map_err(|_| INVALID_HTML)?;
            if offsets[index].replace(value).is_some() {
                return Err(INVALID_HTML.into());
            }
        }
        cursor += end;
        while bytes
            .get(cursor)
            .is_some_and(|byte| matches!(byte, b'\r' | b'\n'))
        {
            cursor += 1;
        }
        if cursor > MAX_CF_HEADER_BYTES {
            return Err(INVALID_HTML.into());
        }
    }
    let [Some(start), Some(end), Some(fragment_start), Some(fragment_end)] = offsets else {
        return Err(INVALID_HTML.into());
    };
    let range = |start: i64, end: i64| -> Result<std::ops::Range<usize>, String> {
        let start = usize::try_from(start).map_err(|_| INVALID_HTML)?;
        let end = usize::try_from(end).map_err(|_| INVALID_HTML)?;
        if start < cursor || start > end || end > bytes.len() || end - start > MAX_HTML_BYTES {
            return Err(INVALID_HTML.into());
        }
        Ok(start..end)
    };
    let fragment = range(fragment_start, fragment_end)?;
    if start != -1 || end != -1 {
        let context = range(start, end)?;
        if fragment.start < context.start || fragment.end > context.end {
            return Err(INVALID_HTML.into());
        }
        std::str::from_utf8(&bytes[context]).map_err(|_| INVALID_HTML)?;
    }
    std::str::from_utf8(&bytes[fragment]).map_err(|_| INVALID_HTML.to_owned())
}

#[cfg(test)]
#[path = "task_transfer_tests.rs"]
mod tests;
