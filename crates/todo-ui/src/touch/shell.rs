use super::{drag_session::DragSession, hub::Hub};
use ferroui_base::input::gesture_recognizers::GestureRecognizer;
use ferroui_base::input::{IPointer, Pointer, PointerEventArgs};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref, StyledElementImpl,
    WeakRef,
};
use ferroui_controls::{Border, Grid, TextBlock};
use ferroui_view::prelude::*;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};
use todo_state::{
    gestures::{TouchTrack, EDGE_WIDTH},
    AppViewModel,
};

#[repr(C)]
pub(super) struct ShellRecognizer {
    pub base: GestureRecognizer,
    pub root: WeakRef<Grid>,
    pub vm: Weak<AppViewModel>,
    pub on_back: Rc<dyn Fn() -> bool>,
    pub can_back: Rc<dyn Fn() -> bool>,
    pub hub: Hub,
    pub session: RefCell<Option<Session>>,
    pub drag: RefCell<Option<DragSession>>,
    pub contacts: RefCell<todo_state::gestures::contacts::Contacts>,
}

ferro_class!(ShellRecognizer: GestureRecognizer);
ferro_impl_classes!(ShellRecognizer: FerroObjectImpl, StyledElementImpl);

pub(super) use todo_state::gestures::shell::ShellGesture as Kind;

pub(super) struct Session {
    pub pointer: Rc<dyn IPointer>,
    pub start: Point,
    pub track: TouchTrack,
    pub back: bool,
    pub drawer: bool,
    pub refresh: bool,
    pub kind: Option<Kind>,
}

pub(crate) fn attach_shell(
    root: &Ref<Grid>,
    vm: Rc<AppViewModel>,
    on_back: Rc<dyn Fn() -> bool>,
    can_back: Rc<dyn Fn() -> bool>,
) {
    let recognizer = instantiate(ShellRecognizer {
        base: GestureRecognizer::construct(),
        root: root.downgrade(),
        vm: Rc::downgrade(&vm),
        on_back,
        can_back,
        hub: Hub::default(),
        session: RefCell::new(None),
        drag: RefCell::new(None),
        contacts: RefCell::default(),
    });
    root.gesture_recognizers().add(&recognizer);
    super::shell_lifetime::attach(root, recognizer, vm);
}

pub(crate) fn shell_feedback(vm: &AppViewModel) -> Element {
    let palette = todo_controls::Palette::for_dark(vm.is_dark());
    stack_panel()
        .name("TouchFeedbackArea")
        .grid_row_span(2)
        .z_index(80)
        .is_hit_test_visible(false)
        .horizontal_alignment(H::Center)
        .vertical_alignment(V::Top)
        .margin((12.0, 52.0, 12.0, 0.0))
        .spacing(8.0)
        .child(
            border()
                .name("TouchGestureFeedback")
                .is_visible(false)
                .horizontal_alignment(H::Center)
                .padding((18.0, 12.0))
                .corner_radius(22.0)
                .background(palette.surface)
                .border_brush(palette.border)
                .border_thickness(1.0)
                .child(
                    text_block()
                        .name("TouchGestureMessage")
                        .font_size(13.0)
                        .foreground(palette.text),
                ),
        )
        .child(super::refresh_feedback::view(vm))
        .into()
}

impl ShellRecognizer {
    /// Safe-area padding can offset TodoApp inside its TopLevel. Every recognizer
    /// reserves this same shell-local edge while movement stays in root coordinates.
    pub fn contains_back_edge(&self, event: &PointerEventArgs) -> bool {
        self.root.upgrade().is_some_and(|root| {
            let point = event.get_position(Some(&root));
            (0.0..=EDGE_WIDTH).contains(&point.x)
        })
    }

    pub fn cancel(&self) {
        let session = self.session.borrow_mut().take();
        let drag = self.drag.borrow_mut().take();
        self.feedback(None);
        let pointer = drag
            .as_ref()
            .map(|drag| drag.pointer.clone())
            .or_else(|| session.as_ref().map(|session| session.pointer.clone()));
        drop(drag);
        if let Some(pointer) = pointer {
            let captured = pointer
                .as_any()
                .downcast_ref::<Pointer>()
                .and_then(Pointer::captured_gesture_recognizer)
                .is_some_and(|recognizer| std::ptr::eq(&*recognizer, &self.base));
            if captured {
                pointer.capture(None);
            }
        }
        self.sync_back();
    }

    pub fn feedback(&self, message: Option<&str>) {
        let Some(root) = self.root.upgrade() else {
            return;
        };
        if let Some(feedback) = root.find_descendant_of_type_where::<Border>(false, |control| {
            control.name().as_deref() == Some("TouchGestureFeedback")
        }) {
            feedback.set_is_visible(message.is_some());
            if let Some(message) = message {
                if let Some(text) = feedback.find_descendant_of_type::<TextBlock>(false) {
                    text.set_text(Some(message.into()));
                }
            }
        }
    }
}
