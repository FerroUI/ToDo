//! Keep the editable draft separate from the native picker's committed date.

use chrono::Datelike;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::DateTime;
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{CalendarDatePicker, TextBox};
use ferroui_view::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use todo_domain::Date;

pub(super) fn attach(
    picker: &Ref<CalendarDatePicker>,
    value: Signal<String>,
    error: Signal<String>,
) {
    // Text parses immediately and requires PART_Calendar. SelectedDate is safe
    // before the template exists; initialize it once, not on form rerenders.
    picker.set_selected_date(native(&value.get_untracked()));
    let draft = Rc::new(Draft {
        picker: picker.downgrade(),
        input: RefCell::new(None),
        changed: RefCell::new(None),
        value,
        error,
        writing: Cell::new(false),
        alive: Cell::new(true),
        rejected: RefCell::new(None),
    });
    let applied = draft.clone();
    let template = picker.template_applied(move |_, event| {
        applied.apply(event.name_scope().find_as::<TextBox>("PART_TextBox"));
    });
    let invalid = draft.clone();
    let validation = picker.date_validation_error(move |event| invalid.reject(event.text()));
    let selected = draft.clone();
    let selection = picker.property_changed(move |event| {
        if event.property() == CalendarDatePicker::selected_date_property().as_property()
            && !selected.writing.get()
        {
            selected.rejected.borrow_mut().take();
            let date = event.get_new_value::<Option<DateTime>>();
            let text = format_date(date);
            selected.update(text.clone(), true);
            selected.write(&text);
        }
    });
    let update = draft.clone();
    effect(move || {
        let value = update.value.get();
        if !update.writing.get() {
            update.rejected.borrow_mut().take();
            update.write(&value);
        }
    });
    on_cleanup(move || {
        draft.alive.set(false);
        draft.detach_input();
        selection.dispose();
        drop(validation);
        if let Some(picker) = draft.picker.upgrade() {
            picker.remove_handler(TemplatedControl::template_applied_event(), template);
        }
    });
}

struct Draft {
    picker: WeakRef<CalendarDatePicker>,
    input: RefCell<Option<WeakRef<TextBox>>>,
    changed: RefCell<Option<Rc<dyn IDisposable>>>,
    value: Signal<String>,
    error: Signal<String>,
    writing: Cell<bool>,
    alive: Cell<bool>,
    rejected: RefCell<Option<String>>,
}

impl Draft {
    fn detach_input(&self) {
        if let Some(subscription) = self.changed.borrow_mut().take() {
            subscription.dispose();
        }
        self.input.borrow_mut().take();
    }

    fn apply(self: &Rc<Self>, input: Option<Ref<TextBox>>) {
        self.detach_input();
        let Some(input) = input else { return };
        *self.input.borrow_mut() = Some(input.downgrade());
        self.write(&self.value.get_untracked());
        let draft = self.clone();
        let weak = input.downgrade();
        let changed = input.property_changed(move |event| {
            if event.property() != TextBox::text_property().as_property() || draft.writing.get() {
                return;
            }
            let Some(input) = weak.upgrade() else { return };
            let text = input.text().unwrap_or_default();
            // Native validation reverts the input to the last committed date.
            // Preserve the rejected draft for the form's Save validation.
            if draft.rejected.borrow().is_some()
                && draft
                    .picker
                    .upgrade()
                    .is_some_and(|picker| text == format_date(picker.selected_date()))
            {
                return;
            }
            draft.rejected.borrow_mut().take();
            draft.update(text, true);
        });
        *self.changed.borrow_mut() = Some(changed);
    }

    fn update(&self, text: String, clear_error: bool) {
        let previous = self.writing.replace(true);
        self.value.set(text);
        if clear_error {
            self.error.set(String::new());
        }
        self.writing.set(previous);
    }

    fn write(&self, text: &str) {
        let Some(input) = self.input.borrow().as_ref().and_then(WeakRef::upgrade) else {
            return;
        };
        let previous = self.writing.replace(true);
        if let Some(picker) = self.picker.upgrade() {
            let selected = native(text);
            if selected.is_some() || text.is_empty() {
                picker.set_selected_date(selected);
            }
        }
        input.set_text(Some(text));
        self.writing.set(previous);
    }

    fn reject(self: &Rc<Self>, text: &str) {
        *self.rejected.borrow_mut() = Some(text.to_owned());
        self.update(text.to_owned(), false);
        self.error
            .set("Enter a valid calendar date in YYYY-MM-DD format.".into());
        let draft = self.clone();
        let rejected = text.to_owned();
        Dispatcher::ui_thread().post_local(
            move || {
                if draft.alive.get()
                    && draft.rejected.borrow().as_deref() == Some(rejected.as_str())
                    && draft.value.get_untracked() == rejected
                {
                    draft.write(&rejected);
                    draft.rejected.borrow_mut().take();
                }
            },
            DispatcherPriority::NORMAL,
        );
    }
}

fn native(text: &str) -> Option<DateTime> {
    let date = text.parse::<Date>().ok()?;
    DateTime::try_new(date.year(), date.month() as i32, date.day() as i32)
}

fn format_date(date: Option<DateTime>) -> String {
    date.map(|date| format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day()))
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "date_field_tests.rs"]
mod tests;
