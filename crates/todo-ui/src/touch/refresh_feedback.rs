use ferroui_base::layout::VerticalAlignment;
use ferroui_controls::automation::AutomationLiveSetting;
use ferroui_view::prelude::*;
use todo_state::AppViewModel;

/// Refresh can outlive its pull gesture while the host synchronizes. Keep this status
/// driven by the VM, independently of the recognizer's transient feedback and capture.
pub(super) fn view(vm: &AppViewModel) -> Element {
    let p = crate::palette(vm);
    let refreshing = vm.refreshing();
    border()
        .name("RefreshFeedback")
        .is_visible(refreshing)
        .is_hit_test_visible(false)
        .padding((18.0, 12.0))
        .corner_radius(22.0)
        .background(p.surface)
        .border_brush(p.border)
        .border_thickness(1.0)
        .child(
            grid()
                .column_definitions("Auto,Auto")
                .column_spacing(10.0)
                .child(
                    progress_ring()
                        .name("RefreshProgress")
                        .width(20.0)
                        .height(20.0)
                        .is_active(refreshing)
                        .is_indeterminate(true)
                        .is_tab_stop(false)
                        .automation_properties_name("Refreshing"),
                )
                .child(
                    text_block()
                        .name("RefreshingLabel")
                        .grid_column(1)
                        .text(if refreshing { "Refreshing…" } else { "" })
                        .automation_properties_live_setting(AutomationLiveSetting::Polite)
                        .font_size(13.0)
                        .foreground(p.text)
                        .vertical_alignment(VerticalAlignment::Center),
                ),
        )
        .into()
}
