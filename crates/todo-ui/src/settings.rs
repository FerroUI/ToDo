mod account;
mod controls;
mod data;

use crate::{detail::edit_field, palette};
use controls::{section_label, toggle};
use ferroui_base::layout::VerticalAlignment as V;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use std::rc::Rc;
use todo_controls::{icon_button, Icon};
use todo_domain::{Action, Theme};
use todo_state::{AppViewModel, HostRequest, Panel};

pub fn view(vm: Rc<AppViewModel>) -> Element {
    rerender(move || content(vm.clone()))
}

fn content(vm: Rc<AppViewModel>) -> Element {
    let p = palette(&vm);
    let settings = vm.state().settings.clone();
    let close = vm.clone();
    let name = vm.clone();
    let mut content = stack_panel().spacing(8.0).margin((20.0, 4.0, 20.0, 24.0))
        .child(section_label("Account", p.text))
        .child(edit_field("AccountName", settings.account_name.clone(), "Your name", false, move |account_name| {
            let mut settings = name.state().settings.clone(); settings.account_name = account_name; name.dispatch(Action::SetSettings(settings));
        }))
        .child(text_block().text("Your tasks are saved on this device. Export a backup to keep an independent copy.")
            .font_size(12.0).foreground(p.secondary).text_wrapping(TextWrapping::Wrap).margin((0.0, 0.0, 0.0, 8.0)))
        .child(section_label("Appearance", p.text));
    for (theme, label) in [
        (Theme::System, "Use system setting"),
        (Theme::Light, "Light theme"),
        (Theme::Dark, "Dark theme"),
    ] {
        let change = vm.clone();
        content = content.child(
            radio_button()
                .content(label)
                .automation_properties_name(label)
                .group_name("Appearance")
                .is_checked(Some(settings.theme == theme))
                .font_size(14.0)
                .min_height(40.0)
                .on_click(move |_| {
                    let mut settings = change.state().settings.clone();
                    settings.theme = theme;
                    change.dispatch(Action::SetSettings(settings));
                    change.request(HostRequest::SetTheme { theme });
                }),
        );
    }
    content = content
        .child(section_label("General", p.text))
        .child(toggle(
            vm.clone(),
            "Confirm before deleting",
            settings.confirm_delete,
            |s, v| s.confirm_delete = v,
        ))
        .child(toggle(
            vm.clone(),
            "Play completion sound",
            settings.completion_sound,
            |s, v| s.completion_sound = v,
        ))
        .child(toggle(
            vm.clone(),
            "Show completed tasks",
            settings.show_completed,
            |s, v| s.show_completed = v,
        ))
        .child(toggle(
            vm.clone(),
            "Week starts on Monday",
            settings.week_starts_monday,
            |s, v| s.week_starts_monday = v,
        ))
        .child(toggle(
            vm.clone(),
            "Recognize dates and times (English)",
            settings.recognize_dates,
            |s, v| s.recognize_dates = v,
        ))
        .child(toggle(
            vm.clone(),
            "Remove recognized dates from titles",
            settings.remove_recognized_dates,
            |s, v| s.remove_recognized_dates = v,
        ))
        .child(section_label("Smart lists", p.text))
        .child(toggle(vm.clone(), "All", settings.show_all, |s, v| {
            s.show_all = v
        }))
        .child(toggle(
            vm.clone(),
            "Completed",
            settings.show_completed_list,
            |s, v| s.show_completed_list = v,
        ))
        .child(toggle(
            vm.clone(),
            "Assigned to me",
            settings.show_assigned,
            |s, v| s.show_assigned = v,
        ))
        .child(toggle(
            vm.clone(),
            "Flagged email",
            settings.show_flagged_email,
            |s, v| s.show_flagged_email = v,
        ))
        .child(account::view(vm.clone()))
        .child(data::view(vm.clone()));
    border()
        .background(p.background)
        .border_brush(p.border)
        .border_thickness((1.0, 0.0, 0.0, 0.0))
        .child(
            grid()
                .row_definitions("52,*")
                .child(
                    grid()
                        .column_definitions("*,Auto")
                        .margin((20.0, 8.0, 12.0, 8.0))
                        .child(
                            text_block()
                                .name("SettingsTitle")
                                .text("Settings")
                                .font_size(18.0)
                                .font_weight(FontWeight::SemiBold)
                                .foreground(p.text)
                                .vertical_alignment(V::Center),
                        )
                        .child(border().grid_column(1).child(icon_button(
                            Icon::Close,
                            "Close settings",
                            move || close.show_panel(Panel::None),
                        ))),
                )
                .child(
                    scroll_viewer()
                        .name("SettingsScroll")
                        .grid_row(1)
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                        .child(content),
                ),
        )
        .into()
}
