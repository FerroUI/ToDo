//! Original filled vector outlines on a 24-unit grid. No platform icon font is required.

mod paths;

use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::Geometry;
use ferroui_base::Ref;
use ferroui_view::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    Sun,
    Star,
    StarFilled,
    Calendar,
    Home,
    List,
    Search,
    Menu,
    Add,
    More,
    ChevronLeft,
    ChevronRight,
    ChevronDown,
    ChevronUp,
    Bell,
    Repeat,
    Attachment,
    Trash,
    Close,
    Check,
    Settings,
    Folder,
    FolderAdd,
    Person,
    People,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Lightbulb,
    Print,
    Sort,
    Share,
    Download,
    Upload,
    Moon,
    Flag,
    Note,
    Clock,
    Copy,
    Undo,
    CheckCircle,
    Circle,
    Info,
    Link,
}

ferroui_base::ferro_thread_local! {
    static GEOMETRIES: RefCell<HashMap<Icon, Ref<Geometry>>> = RefCell::new(HashMap::new());
}

fn geometry(kind: Icon) -> Ref<Geometry> {
    GEOMETRIES.with(|cache| {
        cache
            .borrow_mut()
            .entry(kind)
            .or_insert_with(|| {
                Geometry::parse(paths::data(kind)).expect("valid embedded icon geometry")
            })
            .clone()
    })
}

/// A glyph whose foreground follows its owning button or text style.
pub(crate) fn inherited_icon(kind: Icon, size: f64) -> El<PathIcon> {
    path_icon()
        .data(Some(geometry(kind)))
        .width(size)
        .height(size)
        .horizontal_alignment(HorizontalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Center)
        .is_hit_test_visible(false)
        .automation_properties_is_control_element_override(Some(false))
}

/// A semantic, resolution-independent icon in an explicit color.
pub fn icon(kind: Icon, size: f64, color: &str) -> Element {
    inherited_icon(kind, size).foreground(color).into()
}
