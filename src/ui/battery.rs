use eframe::egui::{self, RichText};

// Battery UI actions
#[derive(Debug, Clone, PartialEq)]
pub struct BatteryAction {
    pub toggle_care: bool,
    pub set_threshold: Option<u8>,
    pub slider_active: Option<bool>,
}

impl Default for BatteryAction {
    fn default() -> Self {
        Self { toggle_care: false, set_threshold: None, slider_active: None }
    }
}

pub fn render_battery_section(
    ui: &mut egui::Ui,
    battery_care: &mut bool,
    current_threshold: &mut u8,
) -> BatteryAction {
    let mut action = BatteryAction::default();

    ui.group(|ui| {
        ui.add(egui::Label::new("🔋 Battery").selectable(false));
        ui.separator();

        ui.horizontal(|ui| {
            if ui.checkbox(battery_care, "Battery Health Optimizer").clicked() {
                action.toggle_care = true;
            }
        });

        // When enabled, show threshold slider (50..=80 step 5), styled like keyboard brightness
        if *battery_care {
            render_threshold_slider(ui, current_threshold, &mut action);
        } else {
            render_battery_status(ui, *battery_care);
        }
    });

    action
}

fn render_battery_status(ui: &mut egui::Ui, battery_care_enabled: bool) {
    ui.horizontal(|ui| {
        let status_text = if battery_care_enabled { "Active" } else { "Disabled" };
        ui.add(egui::Label::new(RichText::new(status_text)).selectable(false));
    });
}

fn render_threshold_slider(ui: &mut egui::Ui, threshold: &mut u8, action: &mut BatteryAction) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new("Threshold:").selectable(false));
        // Map 50..=80 by steps of 5 to a 0..=6 discrete slider, mirroring keyboard brightness style
        let mut step_index: i32 = ((*threshold as i32 - 50) / 5).clamp(0, 6);
        let response = ui.add(
            egui::Slider::new(&mut step_index, 0..=6)
                .custom_formatter(|v, _| format!("{}%", 50 + (v as i32) * 5))
                .custom_parser(|s| s.trim_end_matches('%').parse::<f64>().ok()),
        );
        let new_val = (50 + step_index * 5) as u8;
        let changed = new_val != *threshold;
        *threshold = new_val;

        // Only submit on interaction end to avoid rapid repeated device writes.
        if response.dragged() || response.has_focus() {
            action.slider_active = Some(true);
        } else if response.drag_stopped() || response.lost_focus() {
            action.slider_active = Some(false);
            if changed {
                action.set_threshold = Some(*threshold);
            }
        } else if changed {
            // Click on track or keyboard step without drag: treat as a finalized change.
            action.slider_active = Some(false);
            action.set_threshold = Some(*threshold);
        }
    });
}
