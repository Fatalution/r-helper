use super::RazerGuiApp;
use eframe::egui;
use librazer::types::{FanMode, MaxFanSpeedMode};
use librazer::{command, device::Device};

impl RazerGuiApp {
    pub fn read_current_fan_state(device: &Device) -> (FanMode, Option<u16>) {
        let fan_mode = command::get_perf_mode(device).map(|(_, fm)| fm).unwrap_or(FanMode::Auto);
        let set_rpm = super::get_fan_rpm_set(device, librazer::types::FanZone::Zone1);
        (fan_mode, set_rpm)
    }

    pub fn get_fan_status_from_mode(fan_mode: FanMode, device: &Device) -> (String, Option<u16>) {
        match fan_mode {
            FanMode::Auto => ("Auto".to_string(), None),
            FanMode::Manual => {
                let set_rpm = super::get_fan_rpm_set(device, librazer::types::FanZone::Zone1);
                ("Manual".to_string(), set_rpm)
            }
        }
    }

    pub fn enforce_manual_fan_rpm(&mut self) {
        if self.status.fan_speed == "Manual" {
            if let Some(ref device) = self.device {
                if let Some(current_set_rpm) =
                    super::get_fan_rpm_set(device, librazer::types::FanZone::Zone1)
                {
                    if let Ok(_) = command::set_fan_rpm(device, current_set_rpm, true) {
                        self.manual_fan_rpm = current_set_rpm;
                        self.status.fan_rpm = Some(current_set_rpm);
                        self.last_fan_enforce_time = std::time::Instant::now();
                    }
                }
            }
        }
    }

    pub fn render_fan_section(&mut self, ui: &mut egui::Ui) {
        use crate::ui::fan::render_fan_section;
        let key = egui::Id::new("max_fan_speed_enabled");
        let mut max_enabled = ui.ctx().data(|d| d.get_temp::<bool>(key).unwrap_or(false));
        let (action, new_toggle) = render_fan_section(
            ui,
            &self.status.fan_speed,
            self.status.fan_actual_rpm,
            self.status.fan_rpm,
            &mut self.manual_fan_rpm,
            self.status_messages,
            self.status.performance_mode == "Custom",
            max_enabled,
        );
        if new_toggle != max_enabled && self.status.performance_mode == "Custom" {
            if let Some(ref device) = self.device {
                let result = if new_toggle {
                    command::set_max_fan_speed_mode(device, MaxFanSpeedMode::Enable)
                } else {
                    command::set_max_fan_speed_mode(device, MaxFanSpeedMode::Disable)
                };
                match result {
                    Ok(_) => {
                        max_enabled = new_toggle;
                        self.set_optional_status_message(if new_toggle {
                            "Max fan enabled".into()
                        } else {
                            "Max fan disabled".into()
                        });
                    }
                    Err(e) => self.set_error_message(format!("Failed to toggle max fan: {}", e)),
                }
            }
        }
        ui.ctx().data_mut(|d| d.insert_temp(key, max_enabled));
        match action {
            crate::ui::fan::FanAction::None => {}
            crate::ui::fan::FanAction::SetAutoMode => {
                self.set_fan_mode("auto", None);
            }
            crate::ui::fan::FanAction::SetManualMode(rpm) => {
                self.set_fan_mode("manual", Some(rpm));
            }
            crate::ui::fan::FanAction::SetManualRpm(rpm) => {
                self.set_fan_rpm_only(rpm);
            }
            crate::ui::fan::FanAction::SliderDragging(_) => {}
        }
    }

    pub fn set_fan_mode(&mut self, mode: &str, rpm: Option<u16>) {
        if let Some(ref device) = self.device {
            let result = match mode {
                "auto" => match command::set_fan_mode(device, FanMode::Auto) {
                    Ok(_) => {
                        self.status.fan_speed = "Auto".to_string();
                        self.status.fan_rpm = None;
                        Ok(())
                    }
                    Err(e) => Err(e),
                },
                "manual" => match command::set_fan_mode(device, FanMode::Manual) {
                    Ok(_) => {
                        let rpm_val = rpm.unwrap_or(2000);
                        match command::set_fan_rpm(device, rpm_val, true) {
                            Ok(_) => {
                                self.status.fan_speed = "Manual".to_string();
                                self.status.fan_rpm = Some(rpm_val);
                                Ok(())
                            }
                            Err(e) => Err(e),
                        }
                    }
                    Err(e) => Err(e),
                },
                _ => return,
            };
            match result {
                Ok(_) => {
                    self.set_optional_status_message(format!("Fan set to {} mode", mode));
                }
                Err(e) => {
                    self.set_status_message(format!("Failed to set fan: {}", e));
                }
            }
        } else {
            self.set_no_device_message();
        }
    }

    pub fn set_fan_rpm_only(&mut self, rpm: u16) {
        match crate::utils::execute_device_command_simple(
            self.device.as_ref(),
            |device| command::set_fan_rpm(device, rpm, true),
            &format!("Fans RPM set to: {}", rpm),
            "Failed to set fan RPM",
        ) {
            Ok(message) => {
                self.status.fan_rpm = Some(rpm);
                self.set_optional_status_message(message);
            }
            Err(message) => {
                self.set_error_message(message);
            }
        }
    }
}
