use eframe::egui::{self, RichText};

// Battery UI actions
#[derive(Debug, Clone, PartialEq)]
pub enum BatteryAction {
    // No action
    None,
    // Toggle battery care
    ToggleBatteryCare,
}

pub fn render_battery_section(ui: &mut egui::Ui, battery_care: &mut bool) -> BatteryAction {
    let mut action = BatteryAction::None;

    ui.group(|ui| {
        ui.add(egui::Label::new("🔋 Battery").selectable(false));
        ui.separator();

        ui.horizontal(|ui| {
            if ui.checkbox(battery_care, "Battery Health Optimizer").clicked() {
                action = BatteryAction::ToggleBatteryCare;
            }
        });

        render_battery_status(ui, *battery_care);
    });

    action
}

fn render_battery_status(ui: &mut egui::Ui, battery_care_enabled: bool) {
    ui.horizontal(|ui| {
        let status_text =
            if battery_care_enabled { "Active (Hardware default: 80%)" } else { "Disabled" };
        ui.add(egui::Label::new(RichText::new(status_text)).selectable(false));
    });
}
