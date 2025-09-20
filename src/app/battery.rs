use super::RazerGuiApp;
use eframe::egui;
use librazer::command;

impl RazerGuiApp {
    pub fn toggle_battery_care(&mut self) {
        if let Some(ref device) = self.device {
            let is_on = self.status.battery_care;
            let threshold = self.status.battery_threshold.clamp(50, 80);
            match command::set_battery_care_state_threshold(device, is_on, threshold) {
                Ok(_) => {
                    self.set_optional_status_message(format!(
                        "Battery care {}",
                        if is_on { "enabled" } else { "disabled" }
                    ));
                }
                Err(e) => {
                    self.set_status_message(format!("Failed to set battery care: {}", e));
                    self.status.battery_care = !self.status.battery_care;
                }
            }
        } else {
            self.set_no_device_message();
        }
    }

    pub fn render_battery_section(&mut self, ui: &mut egui::Ui) {
        use crate::ui::battery::render_battery_section as render_batt;
        let action =
            render_batt(ui, &mut self.status.battery_care, &mut self.status.battery_threshold);

        if let Some(active) = action.slider_active {
            self.battery_slider_active = active;
        }

        if action.toggle_care {
            self.toggle_battery_care();
        }

        if let Some(thr) = action.set_threshold {
            if self.device.is_none() {
                self.set_no_device_message();
                return;
            }
            if !self.status.battery_care {
                return;
            }

            // Clamp to supported range and 5% steps; rely on polling to reconcile device state
            let threshold = ((thr.max(50).min(80) - 50) / 5) * 5 + 50;

            if let Some(ref device) = self.device {
                match command::set_battery_care_state_threshold(device, true, threshold) {
                    Ok(_) => {
                        self.status.battery_threshold = threshold;
                        self.set_optional_status_message(format!(
                            "Battery threshold set to {}%",
                            threshold
                        ));
                    }
                    Err(e) => {
                        self.set_error_message(format!("Failed to set battery threshold: {}", e));
                    }
                }
            }
        }
    }
}
