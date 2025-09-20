use crate::device::CompleteDeviceState;
use crate::ui;
use crate::utils::DeviceStateReader;
use anyhow::Result;
use librazer::types::{BatteryCare, LightsAlwaysOn};
use librazer::{command, device::Device};

use super::RazerGuiApp;

impl RazerGuiApp {
    pub fn read_initial_device_state(&mut self) {
        if let Some(ref device) = self.device {
            let mut reader = DeviceStateReader::new(device);
            if let Some(brightness) =
                reader.read(|d| command::get_keyboard_brightness(d), "keyboard brightness")
            {
                self.status.keyboard_brightness = brightness;
                self.temp_brightness_step = ui::lighting::raw_brightness_to_step_index(brightness);
            }
            if let Some((perf_mode, fan_mode)) =
                reader.read(|d| command::get_perf_mode(d), "performance mode")
            {
                self.status.performance_mode =
                    super::RazerGuiApp::perf_mode_to_string(perf_mode).to_string();
                let (fan_speed, fan_rpm) =
                    super::RazerGuiApp::get_fan_status_from_mode(fan_mode, device);
                self.status.fan_speed = fan_speed;
                self.status.fan_rpm = fan_rpm;
                if let Some(rpm) = fan_rpm {
                    self.manual_fan_rpm = rpm;
                }
            }
            if self.status.fan_speed == "Reading..." {
                if let Ok((_, fan_mode)) = command::get_perf_mode(device) {
                    let (fan_speed, fan_rpm) =
                        super::RazerGuiApp::get_fan_status_from_mode(fan_mode, device);
                    self.status.fan_speed = fan_speed;
                    self.status.fan_rpm = fan_rpm;
                    if let Some(rpm) = fan_rpm {
                        self.manual_fan_rpm = rpm;
                    }
                }
            }
            if let Some(lights_always_on) =
                reader.read(|d| command::get_lights_always_on(d), "lights always on")
            {
                self.status.lights_always_on = matches!(lights_always_on, LightsAlwaysOn::Enable);
            }
            if let Some(raw) = reader.read(|d| command::get_battery_care_raw(d), "battery care") {
                let (on, threshold) = command::decode_battery_care(raw);
                self.status.battery_care = on;
                self.status.battery_threshold = threshold.max(50).min(80);
            }
            let errors = reader.finish();
            if !errors.is_empty() && cfg!(debug_assertions) {
                eprintln!("Device state reading errors: {:?}", errors);
            }
        }
    }

    pub fn read_device_status(&mut self) -> Result<()> {
        let device = self.device.as_ref().unwrap();
        let (perf_mode, fan_mode) = command::get_perf_mode(device)?;
        self.status.performance_mode =
            super::RazerGuiApp::perf_mode_to_string(perf_mode).to_string();
        let (fan_speed, fan_rpm) = super::RazerGuiApp::get_fan_status_from_mode(fan_mode, device);
        self.status.fan_speed = fan_speed;
        self.status.fan_rpm = fan_rpm;
        if let Some(rpm) = fan_rpm {
            self.manual_fan_rpm = rpm;
        }
        self.status.fan_actual_rpm =
            super::get_fan_rpm_actual(device, librazer::types::FanZone::Zone1);
        if let Ok(logo_mode) = command::get_logo_mode(device) {
            self.status.logo_mode = super::RazerGuiApp::logo_mode_to_string(logo_mode).to_string();
        }
        if let Ok(brightness) = command::get_keyboard_brightness(device) {
            self.status.keyboard_brightness = brightness;
            self.temp_brightness_step = ui::lighting::raw_brightness_to_step_index(brightness);
        }
        if let Ok(lights_always_on) = command::get_lights_always_on(device) {
            self.status.lights_always_on = matches!(lights_always_on, LightsAlwaysOn::Enable);
        }
        if let Ok(raw) = command::get_battery_care_raw(device) {
            let (on, thr) = command::decode_battery_care(raw);
            self.status.battery_care = on;
            self.status.battery_threshold = thr.max(50).min(80);
        }
        Ok(())
    }

    pub fn sync_ui_with_device_state(&mut self) {
        if let Some(ref device) = self.device {
            if !self.brightness_slider_active {
                if let Ok(brightness) = command::get_keyboard_brightness(device) {
                    self.status.keyboard_brightness = brightness;
                    self.temp_brightness_step =
                        ui::lighting::raw_brightness_to_step_index(brightness);
                }
            }
            let (fan_mode, set_rpm) = super::RazerGuiApp::read_current_fan_state(device);
            let (fan_speed, fan_rpm) =
                super::RazerGuiApp::get_fan_status_from_mode(fan_mode, device);
            self.status.fan_speed = fan_speed;
            self.status.fan_rpm = fan_rpm;
            if let Some(rpm) = set_rpm {
                self.manual_fan_rpm = rpm;
            }
            if let Ok(lights_always_on) = command::get_lights_always_on(device) {
                self.status.lights_always_on = matches!(lights_always_on, LightsAlwaysOn::Enable);
            }
            if !self.battery_slider_active {
                if let Ok(raw) = command::get_battery_care_raw(device) {
                    let (on, thr) = command::decode_battery_care(raw);
                    self.status.battery_care = on;
                    self.status.battery_threshold = thr.max(50).min(80);
                }
            }
        }
    }

    pub fn sync_other_dynamic_state(&mut self) {
        if let Some(ref device) = self.device {
            if let Ok(lights_always_on) = command::get_lights_always_on(device) {
                self.status.lights_always_on = matches!(lights_always_on, LightsAlwaysOn::Enable);
            }
            if !self.battery_slider_active {
                if let Ok(raw) = command::get_battery_care_raw(device) {
                    let (on, thr) = command::decode_battery_care(raw);
                    self.status.battery_care = on;
                    self.status.battery_threshold = thr.max(50).min(80);
                }
            }
        }
    }

    pub fn init_fan_slider_from_device(&mut self) {
        if let Some(ref device) = self.device {
            let (fan_mode, set_rpm) = super::RazerGuiApp::read_current_fan_state(device);
            let (fan_speed, fan_rpm) =
                super::RazerGuiApp::get_fan_status_from_mode(fan_mode, device);
            self.status.fan_speed = fan_speed;
            self.status.fan_rpm = fan_rpm;
            if let Some(rpm) = set_rpm {
                self.manual_fan_rpm = rpm;
            }
        }
    }

    pub fn check_device_state_changes(&mut self) -> Result<()> {
        if let Some(ref device) = self.device {
            let current_state = CompleteDeviceState::read_from_device(device)?;
            if let Some(ref stored_state) = self.device_state {
                if current_state != *stored_state {
                    let old_perf_mode =
                        super::RazerGuiApp::perf_mode_to_string(stored_state.perf_mode);
                    let new_perf_mode =
                        super::RazerGuiApp::perf_mode_to_string(current_state.perf_mode);
                    self.device_state = Some(current_state.clone());
                    self.status.performance_mode =
                        super::RazerGuiApp::perf_mode_to_string(current_state.perf_mode)
                            .to_string();
                    let (fan_speed, fan_rpm) = super::RazerGuiApp::get_fan_status_from_mode(
                        current_state.fan_mode,
                        device,
                    );
                    self.status.fan_speed = fan_speed;
                    self.status.fan_rpm = fan_rpm;
                    if let Some(rpm) = fan_rpm {
                        self.manual_fan_rpm = rpm;
                    }
                    self.status.logo_mode =
                        super::RazerGuiApp::logo_mode_to_string(current_state.logo_mode)
                            .to_string();
                    self.status.keyboard_brightness = current_state.keyboard_brightness;
                    self.temp_brightness_step = ui::lighting::raw_brightness_to_step_index(
                        current_state.keyboard_brightness,
                    );
                    self.status.lights_always_on =
                        matches!(current_state.lights_always_on, LightsAlwaysOn::Enable);
                    self.status.battery_care =
                        matches!(current_state.battery_care, BatteryCare::Enable);
                    self.status.battery_threshold = current_state.battery_threshold;
                    if old_perf_mode != new_perf_mode {
                        self.set_optional_status_message("Mode updated".to_string());
                    } else if self.status_messages {
                        self.set_optional_status_message(
                            "Device state updated externally".to_string(),
                        );
                    }
                }
            } else {
                self.device_state = Some(current_state);
            }
        }
        Ok(())
    }

    pub fn auto_switch_profile(&mut self) {
        if let Some(ref device) = self.device {
            let target_profile =
                if self.ac_power { self.ac_profile.clone() } else { self.battery_profile.clone() };
            let profile_name = if self.ac_power { "AC" } else { "Battery" };
            if let Err(e) = command::set_perf_mode(device, target_profile.perf_mode) {
                self.set_error_message(format!(
                    "Failed to switch to {} profile: {}",
                    profile_name, e
                ));
                return;
            }
            self.status.performance_mode =
                super::RazerGuiApp::perf_mode_to_string(target_profile.perf_mode).to_string();
            self.set_status_message(format!("⚡ Auto-switched to {} profile", profile_name));
        }
        if let Err(_) = self.read_device_status() {
            if let Some(ref device) = self.device {
                let target_profile = if self.ac_power {
                    self.ac_profile.clone()
                } else {
                    self.battery_profile.clone()
                };
                if let Err(e) = self.apply_profile(device, &target_profile) {
                    self.set_error_message(format!("Failed to apply fallback profile: {}", e));
                }
            }
        }
        self.update_stored_device_state();
        self.sync_ui_with_device_state();
    }

    pub fn apply_profile(&self, device: &Device, profile: &CompleteDeviceState) -> Result<()> {
        command::set_perf_mode(device, profile.perf_mode)?;
        command::set_logo_mode(device, profile.logo_mode)?;
        if let Ok(current_brightness) = command::get_keyboard_brightness(device) {
            if current_brightness != profile.keyboard_brightness {
                command::set_keyboard_brightness(device, profile.keyboard_brightness)?;
            }
        } else {
            command::set_keyboard_brightness(device, profile.keyboard_brightness)?;
        }
        command::set_lights_always_on(device, profile.lights_always_on)?;
        let is_on = matches!(profile.battery_care, BatteryCare::Enable);
        let threshold = profile.battery_threshold.clamp(50, 80);
        command::set_battery_care_state_threshold(device, is_on, threshold)?;
        Ok(())
    }
}
