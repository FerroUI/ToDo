use super::*;
use ferroui_base::input::{DataTransfer, IAsyncDataTransferItem, IDataTransferItem};
use ferroui_base::reactive::IDisposable;
use std::cell::Cell;
use std::future::Future;
use std::task::{Context, Poll, Waker};

fn fixture() -> TaskClipboard {
    TaskClipboard::from_json(
        r#"{"kind":"ferroui.todo.tasks","version":1,"token":1,"cut":false,"tasks":[{"id":2,"list_id":1,"title":"東京 & <x> 😀","notes":"A retained note","important":true,"created_on":"2026-11-23","updated_on":"2026-11-23"},{"id":3,"list_id":1,"title":"Second task","created_on":"2026-11-23","updated_on":"2026-11-23"}]}"#,
    )
    .unwrap()
}

fn ready<F: Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("portable DataTransfer reads must complete synchronously"),
    }
}

fn read_both(data: &DataTransfer) -> Result<(Option<TaskClipboard>, String), String> {
    let drop_result = read_drop(data);
    assert_eq!(drop_result, ready(read_clipboard(data)));
    drop_result
}

fn one_item(item: Rc<DataTransferItem>) -> Rc<DataTransfer> {
    let data = DataTransfer::new();
    data.add(item);
    data
}

fn html_item(format: &str, html: &str) -> Rc<DataTransferItem> {
    if format == "HTML Format" {
        DataTransferItem::create(
            &windows_html_format(),
            Some(Rc::<[u8]>::from(encode_cf_html(html).unwrap())),
        )
    } else {
        DataTransferItem::create(&html_format(format), Some(html.to_owned()))
    }
}

#[test]
fn native_offer_has_three_representations_and_round_trips_sync_and_async() {
    let tasks = fixture();
    let offered = item(&tasks).unwrap();
    assert_eq!(offered.formats().len(), 3);
    assert_eq!(
        offered.try_get_value(&DataFormat::text()),
        Some(tasks.text())
    );
    assert!(offered.contains(&task_format()));
    let native_html: DataFormat = if cfg!(target_os = "windows") {
        windows_html_format().into()
    } else if cfg!(target_os = "macos") {
        html_format("public.html").into()
    } else {
        html_format("text/html").into()
    };
    assert!(offered.contains(&native_html));
    let data = one_item(offered);
    assert!(has_supported(data.as_ref()));
    assert_eq!(
        read_both(&data).unwrap(),
        (Some(tasks.clone()), tasks.text())
    );
}

#[test]
fn task_bytes_win_across_items_without_reading_weaker_lazy_representations() {
    let tasks = fixture();
    let weaker = DataTransferItem::new();
    weaker.set_with(&html_format("text/html"), || -> Option<String> {
        panic!("HTML must not be fetched when task bytes are available")
    });
    weaker.set_with(&DataFormat::text(), || -> Option<String> {
        panic!("plain text must not be fetched when task bytes are available")
    });
    let data = DataTransfer::new();
    data.add(weaker);
    data.add(item(&tasks).unwrap());
    assert_eq!(read_both(&data).unwrap().0, Some(tasks));
}

#[test]
fn marked_html_fallback_works_for_browser_linux_mac_and_windows_formats() {
    let tasks = fixture();
    for format in ["text/html", "public.html", "HTML Format"] {
        let offered = html_item(format, &tasks.to_html().unwrap());
        offered.set_text(Some("A stale plain-text representation"));
        let data = one_item(offered);
        assert!(has_supported(data.as_ref()));
        assert_eq!(
            read_both(&data).unwrap(),
            (Some(tasks.clone()), tasks.text())
        );
    }
}

#[test]
fn ordinary_external_html_falls_back_to_text_without_importing_markup() {
    for format in ["text/html", "public.html", "HTML Format"] {
        let offered = html_item(
            format,
            "<script>throw 'never run'</script><p>Formatted task</p>",
        );
        offered.set_text(Some("First task\nSecond task"));
        assert_eq!(
            read_both(&one_item(offered)).unwrap(),
            (None, "First task\nSecond task".into())
        );
    }
    // Unmarked external CF_HTML is not required to be parseable to use its text.
    let offered = DataTransferItem::create(
        &windows_html_format(),
        Some(Rc::<[u8]>::from(
            &b"<p>Unmarked HTML without a CF header</p>"[..],
        )),
    );
    offered.set_text(Some("Plain task"));
    assert_eq!(
        read_both(&one_item(offered)).unwrap(),
        (None, "Plain task".into())
    );
}

#[test]
fn malformed_or_unreadable_task_data_does_not_silently_fall_back() {
    for bytes in [b"not JSON".to_vec(), vec![0xff, 0xfe]] {
        let offered = DataTransferItem::create(&task_format(), Some(Rc::<[u8]>::from(bytes)));
        offered.set(
            &html_format("text/html"),
            Some(fixture().to_html().unwrap()),
        );
        offered.set_text(Some("Do not import this fallback"));
        assert!(read_both(&one_item(offered)).is_err());
    }
    let wrong_type = DataTransferItem::create(
        &DataFormat::create_string_application_format("FerroUI.ToDo.Tasks"),
        Some(fixture().to_json().unwrap()),
    );
    wrong_type.set_text(Some("Do not import this fallback"));
    assert!(read_both(&one_item(wrong_type)).is_err());
    let unavailable = DataTransferItem::create_with(&task_format(), || None);
    unavailable.set_text(Some("Do not import this fallback"));
    assert!(read_both(&one_item(unavailable)).is_err());
}

#[test]
fn malformed_marked_html_does_not_silently_fall_back_on_any_platform() {
    for format in ["text/html", "public.html", "HTML Format"] {
        let offered = html_item(format, "<ul data-ferroui-todo-v1=\"not-hex\"></ul>");
        offered.set_text(Some("Do not import this fallback"));
        assert!(read_both(&one_item(offered)).is_err());
    }
}

#[test]
fn serialized_text_and_html_limits_are_enforced_before_decoding() {
    let large_tasks = DataTransferItem::create(
        &task_format(),
        Some(Rc::<[u8]>::from(vec![b' '; MAX_TASK_CLIPBOARD_BYTES + 1])),
    );
    assert!(read_both(&one_item(large_tasks))
        .unwrap_err()
        .contains("4 MiB"));
    let large_text = DataTransferItem::create_text(Some(&"x".repeat(MAX_TASK_CLIPBOARD_BYTES + 1)));
    assert!(read_both(&one_item(large_text))
        .unwrap_err()
        .contains("4 MiB"));
    for format in ["text/html", "public.html"] {
        let large_html =
            DataTransferItem::create(&html_format(format), Some("x".repeat(MAX_HTML_BYTES + 1)));
        assert!(read_both(&one_item(large_html))
            .unwrap_err()
            .contains("8 MiB"));
    }
    let large_cf_html = DataTransferItem::create(
        &windows_html_format(),
        Some(Rc::<[u8]>::from(vec![
            b' ';
            MAX_HTML_BYTES
                + MAX_CF_HEADER_BYTES
                + 1
        ])),
    );
    assert!(read_both(&one_item(large_cf_html))
        .unwrap_err()
        .contains("8 MiB"));
}

#[test]
fn html_expansion_is_bounded_when_creating_an_offer() {
    let json = fixture()
        .to_json()
        .unwrap()
        .replace("東京 & <x> 😀", &"&".repeat(MAX_HTML_BYTES / 7));
    let large = TaskClipboard::from_json(&json).unwrap();
    assert!(item(&large).err().unwrap().contains("8 MiB"));
}

#[test]
fn plain_text_items_are_joined_in_source_order_and_unrelated_formats_are_ignored() {
    let data = DataTransfer::new();
    data.add(DataTransferItem::create_text(Some("First\nSecond")));
    data.add(DataTransferItem::create(
        &DataFormat::create_bytes_platform_format("application/octet-stream"),
        Some(Rc::<[u8]>::from(&b"not task text"[..])),
    ));
    data.add(DataTransferItem::create_text(Some("")));
    data.add(DataTransferItem::create_text(Some("Third\r\nFourth")));
    assert_eq!(
        read_both(&data).unwrap(),
        (None, "First\nSecond\nThird\r\nFourth".into())
    );
}

#[test]
fn text_item_count_and_aggregate_size_have_independent_limits() {
    let data = DataTransfer::new();
    for _ in 0..MAX_TEXT_ITEMS {
        data.add(DataTransferItem::create_text(Some("Task")));
    }
    assert_eq!(read_both(&data).unwrap().1.lines().count(), MAX_TEXT_ITEMS);
    data.add(DataTransferItem::create_text(Some("Task")));
    assert!(read_both(&data).unwrap_err().contains("1,000"));
    let data = DataTransfer::new();
    let half = "x".repeat(MAX_TASK_CLIPBOARD_BYTES / 2);
    data.add(DataTransferItem::create_text(Some(&half)));
    data.add(DataTransferItem::create_text(Some(&half)));
    assert!(read_both(&data).unwrap_err().contains("4 MiB"));
}

struct MetadataOnly(Rc<[DataFormat]>);

impl IDisposable for MetadataOnly {
    fn dispose(&self) {
        panic!("hover must not dispose its caller's transfer")
    }
}

impl IDataTransfer for MetadataOnly {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.0.clone()
    }
    fn items(&self) -> Rc<[Rc<dyn IDataTransferItem>]> {
        panic!("hover must not enumerate or read items")
    }
}

#[test]
fn hover_inspects_only_format_metadata_and_does_not_accept_files() {
    for (_, format) in formats() {
        assert!(has_supported(&MetadataOnly(Rc::from([format]))));
    }
    assert!(!has_supported(&MetadataOnly(Rc::from([
        DataFormat::file().into(),
        DataFormat::bitmap().into(),
    ]))));
}

fn header_offset(bytes: &[u8], name: &str) -> usize {
    std::str::from_utf8(bytes)
        .unwrap()
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{name}:")))
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

fn replace_offset(bytes: &mut [u8], name: &str, value: i64) {
    let field = format!("{name}:");
    let start = std::str::from_utf8(bytes).unwrap().find(&field).unwrap() + field.len();
    let replacement = format!("{value:010}");
    assert_eq!(replacement.len(), 10);
    bytes[start..start + 10].copy_from_slice(replacement.as_bytes());
}

#[test]
fn cf_html_offsets_count_utf8_bytes_and_omit_the_nul_terminator() {
    let html = fixture().to_html().unwrap();
    let bytes = encode_cf_html(&html).unwrap();
    let start = header_offset(&bytes, "StartHTML");
    let end = header_offset(&bytes, "EndHTML");
    let fragment_start = header_offset(&bytes, "StartFragment");
    let fragment_end = header_offset(&bytes, "EndFragment");
    assert_eq!(&bytes[start..start + 6], b"<html>");
    assert_eq!(end, bytes.len() - 1);
    assert_eq!(bytes[end], 0);
    assert_eq!(&bytes[fragment_start..fragment_end], html.as_bytes());
    assert_eq!(fragment_end - fragment_start, html.len());
    assert_ne!(html.len(), html.encode_utf16().count());
    assert_eq!(
        &bytes[fragment_start - 20..fragment_start],
        b"<!--StartFragment-->"
    );
    assert_eq!(
        &bytes[fragment_end..fragment_end + 18],
        b"<!--EndFragment-->"
    );
    assert_eq!(cf_html_fragment(&bytes).unwrap(), html);
}

#[test]
fn cf_html_rejects_invalid_ranges_and_partial_utf8_characters() {
    let html = fixture().to_html().unwrap();
    let original = encode_cf_html(&html).unwrap();
    let start = header_offset(&original, "StartFragment");
    let end = header_offset(&original, "EndFragment");
    for (field, value) in [
        ("StartFragment", -1),
        ("StartFragment", end as i64 + 1),
        ("EndFragment", original.len() as i64 + 1),
        ("EndHTML", start as i64),
        ("StartHTML", -1), // Only one context offset is absent.
        (
            "StartFragment",
            (start + html.find('東').unwrap() + 1) as i64,
        ),
    ] {
        let mut bytes = original.clone();
        replace_offset(&mut bytes, field, value);
        assert!(
            cf_html_fragment(&bytes).is_err(),
            "accepted {field}={value}"
        );
        let offered =
            DataTransferItem::create(&windows_html_format(), Some(Rc::<[u8]>::from(bytes)));
        offered.set_text(Some("Do not import this fallback"));
        assert!(read_both(&one_item(offered)).is_err());
    }
    let mut fragment_only = original;
    replace_offset(&mut fragment_only, "StartHTML", -1);
    replace_offset(&mut fragment_only, "EndHTML", -1);
    assert_eq!(cf_html_fragment(&fragment_only).unwrap(), html);
}

#[test]
fn cf_html_ignores_task_markers_outside_the_selected_fragment() {
    let mut bytes = encode_cf_html(&fixture().to_html().unwrap()).unwrap();
    let end = header_offset(&bytes, "EndFragment");
    replace_offset(&mut bytes, "StartFragment", end as i64);
    let offered = DataTransferItem::create(&windows_html_format(), Some(Rc::<[u8]>::from(bytes)));
    offered.set_text(Some("Selected ordinary text"));
    assert_eq!(
        read_both(&one_item(offered)).unwrap(),
        (None, "Selected ordinary text".into())
    );
}

struct CallerOwned {
    data: Rc<DataTransfer>,
    disposed: Rc<Cell<bool>>,
}

impl IDisposable for CallerOwned {
    fn dispose(&self) {
        self.disposed.set(true);
    }
}

impl IDataTransfer for CallerOwned {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.data.formats()
    }
    fn items(&self) -> Rc<[Rc<dyn IDataTransferItem>]> {
        IDataTransfer::items(self.data.as_ref())
    }
}

impl IAsyncDataTransfer for CallerOwned {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.data.formats()
    }
    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        IAsyncDataTransfer::items(self.data.as_ref())
    }
}

#[test]
fn borrowed_reads_materialize_values_without_disposing_the_transfer() {
    let tasks = fixture();
    let disposed = Rc::new(Cell::new(false));
    let lease = disposed.clone();
    let json = tasks.to_json().unwrap();
    let offered = DataTransferItem::create_with(&task_format(), move || {
        assert!(
            !lease.get(),
            "payload read after the native drop lease ended"
        );
        Some(Rc::<[u8]>::from(json.as_bytes()))
    });
    let data = CallerOwned {
        data: one_item(offered),
        disposed,
    };
    let dropped = read_drop(&data).unwrap();
    let pasted = ready(read_clipboard(&data)).unwrap();
    assert!(!data.disposed.get());
    data.dispose();
    assert_eq!(dropped, (Some(tasks.clone()), tasks.text()));
    assert_eq!(pasted, dropped);
}

#[cfg(panic = "unwind")]
mod failing_drop {
    use super::*;

    #[derive(Clone, Copy)]
    enum Failure {
        TransferFormats,
        Items,
        Formats,
        Payload,
    }

    struct Transfer(Failure);
    struct Item(Failure);

    impl IDisposable for Transfer {
        fn dispose(&self) {
            panic!("the failed read must leave disposal to its caller")
        }
    }

    impl IDataTransfer for Transfer {
        fn formats(&self) -> Rc<[DataFormat]> {
            if matches!(self.0, Failure::TransferFormats) {
                panic!("private provider metadata error")
            }
            Rc::from([task_format().into()])
        }

        fn items(&self) -> Rc<[Rc<dyn IDataTransferItem>]> {
            if matches!(self.0, Failure::Items) {
                panic!("private provider item error")
            }
            Rc::from([Rc::new(Item(self.0)) as Rc<dyn IDataTransferItem>])
        }
    }

    impl IDataTransferItem for Item {
        fn formats(&self) -> Rc<[DataFormat]> {
            if matches!(self.0, Failure::Formats) {
                panic!("private provider format error")
            }
            Rc::from([task_format().into()])
        }

        fn try_get_raw(&self, _: &DataFormat) -> Option<Rc<dyn Any>> {
            panic!("private provider payload error")
        }
    }

    #[test]
    fn native_format_metadata_failures_decline_the_hover() {
        assert!(!has_supported(&Transfer(Failure::TransferFormats)));
    }

    #[test]
    fn synchronous_provider_failures_become_a_stable_error_at_every_read_boundary() {
        for failure in [Failure::Items, Failure::Formats, Failure::Payload] {
            assert_eq!(
                read_drop(&Transfer(failure)).unwrap_err(),
                "The dropped data could not be read. Please try again."
            );
        }
    }
}
