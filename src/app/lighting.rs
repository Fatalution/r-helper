use super::RazerGuiApp;
use crate::ui::lighting;
use eframe::egui;
use librazer::command;
use librazer::types::{LightsAlwaysOn, LogoMode};

impl RazerGuiApp {
    pub fn logo_mode_to_string(mode: LogoMode) -> &'static str {
        match mode {
            LogoMode::Static => "Static",
            LogoMode::Breathing => "Breathing",
            LogoMode::Off => "Off",
        }
    }
    pub fn string_to_logo_mode(mode: &str) -> Option<LogoMode> {
        match mode {
            "Static" => Some(LogoMode::Static),
            "Breathing" => Some(LogoMode::Breathing),
            "Off" => Some(LogoMode::Off),
            _ => None,
        }
    }

    pub fn set_logo_mode(&mut self, mode: &str) {
        let logo_mode = match Self::string_to_logo_mode(mode) {
            Some(mode) => mode,
            None => return,
        };
        match crate::utils::execute_device_command_simple(
            self.device.as_ref(),
            |device| command::set_logo_mode(device, logo_mode),
            &format!("Logo mode set to {}", mode),
            "Failed to set logo mode",
        ) {
            Ok(message) => {
                self.status.logo_mode = mode.to_string();
                self.set_optional_status_message(message);
            }
            Err(message) => {
                self.set_error_message(message);
            }
        }
    }

    pub fn set_brightness(&mut self, brightness: u8) {
        match crate::utils::execute_device_command_simple(
            self.device.as_ref(),
            |device| command::set_keyboard_brightness(device, brightness),
            &format!(
                "Brightness set to step {}",
                lighting::raw_brightness_to_step_index(brightness)
            ),
            "Failed to set brightness",
        ) {
            Ok(message) => {
                self.status.keyboard_brightness = brightness;
                self.temp_brightness_step = lighting::raw_brightness_to_step_index(brightness);
                self.set_optional_status_message(message);
            }
            Err(message) => {
                self.set_error_message(message);
            }
        }
    }

    pub fn toggle_lights_always_on(&mut self) {
        let lights_always_on = if self.status.lights_always_on {
            LightsAlwaysOn::Enable
        } else {
            LightsAlwaysOn::Disable
        };
        if let Some(ref device) = self.device {
            match command::set_lights_always_on(device, lights_always_on) {
                Ok(_) => {
                    self.set_optional_status_message(format!(
                        "Keyboard Backlight Always On {}",
                        if self.status.lights_always_on { "enabled" } else { "disabled" }
                    ));
                    self.update_stored_device_state();
                }
                Err(e) => {
                    self.set_status_message(format!("Failed to set lights always on: {}", e));
                    self.status.lights_always_on = !self.status.lights_always_on;
                }
            }
        } else {
            self.set_no_device_message();
        }
    }

    pub fn render_lighting_section(&mut self, ui: &mut egui::Ui) {
        use crate::ui::lighting::render_lighting_section;
        let action = render_lighting_section(
            ui,
            &self.status.logo_mode,
            &mut self.temp_brightness_step,
            &mut self.status.lights_always_on,
        );
        if let Some(active) = action.slider_active {
            self.brightness_slider_active = active;
        }
        if let Some(mode) = action.logo_mode {
            self.set_logo_mode(&mode);
        }
        if let Some(brightness) = action.brightness {
            self.set_brightness(brightness);
        }
        if action.lights_always_on {
            self.toggle_lights_always_on();
        }
    }
}
