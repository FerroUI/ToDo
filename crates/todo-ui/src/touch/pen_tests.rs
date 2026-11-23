use super::*;
use crate::touch::{contacts, hub::Hub};
use ferroui_base::input::gesture_recognizers::GestureRecognizer;
use ferroui_base::input::{
    FocusManager, IPointerDevice, InputElement, InputManager, PenDevice, TouchDevice,
};
use ferroui_base::reactive::{ObservableExt, Subscription};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{instantiate, Ref};
use ferroui_controls::{Button, Grid};
use std::cell::{Cell, RefCell};

struct Root(Ref<InputElement>);
impl IInputRoot for Root {
    fn focus_manager(&self) -> Option<Rc<FocusManager>> {
        None
    }
    fn pointer_over_element(&self) -> Option<Ref<InputElement>> {
        None
    }
    fn set_pointer_over_element(&self, _: Option<Ref<InputElement>>) {}
    fn cursor_element(&self) -> Option<Ref<InputElement>> {
        None
    }
    fn set_cursor_element(&self, _: Option<Ref<InputElement>>) {}
    fn root_element(&self) -> Ref<InputElement> {
        self.0.clone()
    }
    fn focus_root(&self) -> Ref<InputElement> {
        self.0.clone()
    }
    fn pointer_over_invalidated(&self) {}
}

#[test]
fn native_pen_cancellation_disposes_pressed_button_without_release_or_click() {
    let _dispatcher = Dispatcher::unit_test_scope();
    let _scope = Subscription::new(FerroLocator::enter_scope());
    let manager = Rc::new(InputManager::new());
    FerroLocator::current_mutable()
        .bind::<dyn IInputManager>()
        .to_constant(manager.clone());
    let shell = Grid::new();
    let recognizer = instantiate(ShellRecognizer {
        base: GestureRecognizer::construct(),
        root: shell.downgrade(),
        vm: Weak::new(),
        on_back: Rc::new(|| false),
        can_back: Rc::new(|| false),
        hub: Hub::default(),
        session: RefCell::new(None),
        drag: RefCell::new(None),
        contacts: RefCell::default(),
        pens: RefCell::default(),
    });
    let weak = recognizer.downgrade();
    let _raw = manager.pre_process().subscribe_fn_guard(move |event| {
        if let (Some(recognizer), Some(pointer)) =
            (weak.upgrade(), event.downcast_ref::<RawPointerEventArgs>())
        {
            contacts::process(&recognizer, pointer);
        }
    });
    let button = Button::new();
    let root: Rc<dyn IInputRoot> = Rc::new(Root(button.clone().upcast()));
    let releases = Rc::new(Cell::new(0));
    let released = releases.clone();
    button.add_handler(InputElement::pointer_released_event(), move |_, _| {
        released.set(released.get() + 1)
    });
    let clicks = Rc::new(Cell::new(0));
    let clicked = clicks.clone();
    button.click(move |_, _| clicked.set(clicked.get() + 1));
    let pen = PenDevice::new(false); // Desktop pens may remain allocated while hovering.
    let send = |device: Rc<dyn IInputDevice>, id, kind, modifiers| {
        let event = Rc::new(RawPointerEventArgs::new(
            device,
            100,
            root.clone(),
            kind,
            Point::new(1.0, 1.0),
            modifiers,
        ));
        event.set_raw_pointer_id(id);
        event.set_input_hit_test_result(
            Some(button.clone().upcast()),
            Some(button.clone().upcast()),
        );
        manager.process_input(event.clone());
        event
    };
    let down = send(
        pen.clone(),
        7,
        RawPointerEventType::LeftButtonDown,
        RawInputModifiers::LEFT_MOUSE_BUTTON,
    );
    assert!(button.is_pressed());
    assert!(pen.try_get_pointer(&down).unwrap().captured().is_some());
    assert!(cancel(&recognizer, Some(7)));
    assert!(!button.is_pressed());
    assert!(pen.try_get_pointer(&down).is_none());
    assert!(!recognizer.contacts.borrow().has_pen(7));
    assert!(
        !cancel(&recognizer, Some(7)),
        "duplicate host cancellation is inert"
    );
    send(
        pen.clone(),
        7,
        RawPointerEventType::LeftButtonUp,
        RawInputModifiers::NONE,
    );
    assert_eq!(
        (releases.get(), clicks.get()),
        (0, 0),
        "cancel and late Up never release or click a native button"
    );

    let next = send(
        pen.clone(),
        8,
        RawPointerEventType::LeftButtonDown,
        RawInputModifiers::LEFT_MOUSE_BUTTON,
    );
    assert!(button.is_pressed(), "a fresh pen contact remains usable");
    let touch = send(
        TouchDevice::new(),
        8,
        RawPointerEventType::TouchBegin,
        RawInputModifiers::NONE,
    );
    assert!(
        touch.handled(),
        "mixed touch cannot press the button under the existing pen"
    );
    assert!(!button.is_pressed());
    assert!(
        pen.try_get_pointer(&next).is_none(),
        "mixed input disposes the real pen's capture"
    );
    send(
        pen,
        8,
        RawPointerEventType::LeftButtonUp,
        RawInputModifiers::NONE,
    );
    assert_eq!((releases.get(), clicks.get()), (0, 0));
}
