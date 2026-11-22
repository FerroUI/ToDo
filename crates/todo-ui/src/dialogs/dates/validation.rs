use ferroui_base::media::TextWrapping;
use ferroui_controls::automation::AutomationLiveSetting;
use ferroui_view::prelude::*;

pub(super) fn message(message: String, dark: bool) -> Element {
    text_block()
        .name("ScheduleValidation")
        .is_visible(!message.is_empty())
        .text(message)
        .font_size(12.0)
        .text_wrapping(TextWrapping::Wrap)
        .foreground(if dark { "#F1707B" } else { "#D13438" })
        .automation_properties_live_setting(AutomationLiveSetting::Assertive)
        .into()
}
