use super::RazerGuiApp;
use eframe::egui;
use librazer::command;
use librazer::types::BatteryCare;

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
        use crate::ui::battery::render_battery_section;
        let action = render_battery_section(
            ui,
            &mut self.status.battery_care,
            &mut self.status.battery_threshold,
        );
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
            let threshold = thr.clamp(50, 80);
            let original = self.status.battery_threshold;
            let (new_care, new_thr, err_msg, ok_msg) = {
                let device = self.device.as_ref().unwrap();
                let mut new_care: Option<bool> = None;
                let new_thr: Option<u8>;
                let mut err_msg: Option<String> = None;
                let mut ok_msg: Option<String> = None;
                match command::set_battery_care_state_threshold(device, true, threshold) {
                    Err(e) => {
                        err_msg = Some(format!("Failed to set battery threshold: {}", e));
                        if let Ok((mode, read_thr)) = command::get_battery_care_state(device) {
                            new_care = Some(matches!(mode, BatteryCare::Enable));
                            new_thr = Some(read_thr.max(50).min(80));
                        } else {
                            new_thr = Some(original);
                        }
                    }
                    Ok(_) => match command::get_battery_care_state(device) {
                        Ok((mode, read_thr)) => {
                            let on = matches!(mode, BatteryCare::Enable);
                            if on && read_thr == threshold {
                                new_thr = Some(threshold);
                                ok_msg = Some(format!("Battery threshold set to {}%", threshold));
                            } else {
                                new_care = Some(on);
                                new_thr = Some(read_thr.max(50).min(80));
                                err_msg = Some(format!(
                                    "Battery threshold mismatch (set {}%, device {}%)",
                                    threshold, read_thr
                                ));
                            }
                        }
                        Err(_) => {
                            new_thr = Some(original);
                            err_msg = Some("Failed to confirm battery threshold".to_string());
                        }
                    },
                }
                (new_care, new_thr, err_msg, ok_msg)
            };
            if let Some(on) = new_care {
                self.status.battery_care = on;
            }
            if let Some(t) = new_thr {
                self.status.battery_threshold = t;
            }
            if let Some(m) = err_msg {
                self.set_error_message(m);
            } else if let Some(m) = ok_msg {
                self.set_optional_status_message(m);
            }
        }
    }
}
