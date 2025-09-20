use super::{InitMessage, RazerGuiApp};
use crate::power::get_power_state;
use crate::system::get_system_specs;
use crate::ui;
use eframe::egui;
use librazer::{command, device::Device};
use strum::IntoEnumIterator;

impl RazerGuiApp {
    pub fn start_device_detection(&mut self, sender: std::sync::mpsc::Sender<InitMessage>) {
        self.detecting_device = true;
        std::thread::spawn(move || {
            let present = match Device::detect() {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("Failed to connect to Razer device: {}", e);
                    false
                }
            };
            let _ = sender.send(InitMessage::DeviceDetectionComplete(present));
        });
    }

    pub fn start_background_initialization(
        &mut self,
        sender: std::sync::mpsc::Sender<InitMessage>,
    ) {
        let device_name: Option<String> = None;
        std::thread::spawn(move || {
            if let Ok(ac_power) = get_power_state() {
                let _ = sender.send(InitMessage::PowerStateRead(ac_power));
            }
            let _ = sender.send(InitMessage::InitializationComplete);
            let system_specs = get_system_specs(device_name.as_deref());
            let _ = sender.send(InitMessage::SystemSpecsComplete(system_specs));
        });
        self.loading = false;
    }

    pub fn process_background_initialization(&mut self) {
        let mut messages_to_process = Vec::new();
        if let Some(ref receiver) = self.init_receiver {
            while let Ok(message) = receiver.try_recv() {
                messages_to_process.push(message);
            }
        }
        for message in messages_to_process {
            match message {
                InitMessage::DeviceDetectionComplete(present) => {
                    self.device_detection_done = true;
                    if present {
                        self.detecting_device = false;
                    }
                    if present {
                        if let Ok(dev) = Device::detect() {
                            self.device = Some(dev);
                        }
                    }
                    self.detect_available_performance_modes();
                    if self.device.is_some() {
                        self.read_initial_device_state();
                        self.set_status_message("Initializing...".to_string());
                    }
                }
                InitMessage::SystemSpecsComplete(specs) => {
                    self.system_specs = specs;
                    self.init_specs_complete = true;
                    if self.fully_initialized && self.init_power_read && self.init_specs_complete {
                        if self.device.is_some() {
                            self.set_status_message("Initialization complete".to_string());
                        } else {
                            self.set_optional_status_message("Initialization complete".to_string());
                        }
                    } else {
                        self.set_optional_status_message(
                            "System specifications loaded".to_string(),
                        );
                    }
                }
                InitMessage::PowerStateRead(ac_power) => {
                    self.ac_power = ac_power;
                    self.init_power_read = true;
                }
                InitMessage::InitializationComplete => {
                    self.fully_initialized = true;
                    if self.device.is_some() {
                        if let Err(e) = self.read_device_status() {
                            self.set_error_message(format!("Failed to read device status: {}", e));
                        } else {
                            self.update_stored_device_state();
                            self.sync_ui_with_device_state();
                            self.init_fan_slider_from_device();
                        }
                    }
                }
            }
        }
    }

    pub fn update_impl(&mut self, ctx: &egui::Context) {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        self.process_background_initialization();
        let hidden_on =
            ctx.data(|d| d.get_temp::<bool>("perf_hidden_show".into()).unwrap_or(false));
        if self.device.is_some() {
            if hidden_on {
                self.available_performance_modes = librazer::types::PerfMode::iter().collect();
            } else {
                self.detect_available_performance_modes();
            }
        }
        self.message_manager.update();
        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
        if minimized && self.fully_initialized {
            const PERF_POLL_INTERVAL: f32 = 2.5;
            if self.last_perf_poll_time.elapsed().as_secs_f32() >= PERF_POLL_INTERVAL {
                if let Some(ref device) = self.device {
                    if let Ok((perf_mode, fan_mode)) = command::get_perf_mode(device) {
                        let new_mode =
                            super::RazerGuiApp::perf_mode_to_string(perf_mode).to_string();
                        if self.status.performance_mode != new_mode {
                            self.status.performance_mode = new_mode;
                            let (fan_speed, fan_rpm) =
                                super::RazerGuiApp::get_fan_status_from_mode(fan_mode, device);
                            self.status.fan_speed = fan_speed;
                            self.status.fan_rpm = fan_rpm;
                        }
                    }
                }
                self.last_perf_poll_time = std::time::Instant::now();
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) {
            self.should_quit = true;
        }
        if self.should_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if !ctx.input(|i| i.viewport().minimized.unwrap_or(false)) {
            if self.fully_initialized {
                const AUTO_REFRESH_INTERVAL: f32 = 0.5;
                if self.last_refresh_time.elapsed().as_secs_f32() >= AUTO_REFRESH_INTERVAL {
                    if self.device.is_some() && !self.loading {
                        if let Ok(current_ac_power) = get_power_state() {
                            if current_ac_power != self.ac_power {
                                self.ac_power = current_ac_power;
                                self.auto_switch_profile();
                            }
                        }
                        if let Some(ref device) = self.device {
                            self.status.fan_actual_rpm =
                                super::get_fan_rpm_actual(device, librazer::types::FanZone::Zone1);
                            let (current_fan_mode, _) =
                                super::RazerGuiApp::read_current_fan_state(device);
                            let (fan_speed, _) = super::RazerGuiApp::get_fan_status_from_mode(
                                current_fan_mode,
                                device,
                            );
                            self.status.fan_speed = fan_speed;
                        }
                        if self.last_fan_enforce_time.elapsed().as_secs_f32() >= 1.0 {
                            self.enforce_manual_fan_rpm();
                        }
                        if let Some(ref device) = self.device {
                            if !self.brightness_slider_active {
                                if let Ok(brightness) = command::get_keyboard_brightness(device) {
                                    self.status.keyboard_brightness = brightness;
                                    self.temp_brightness_step =
                                        ui::lighting::raw_brightness_to_step_index(brightness);
                                }
                            }
                        }
                        self.sync_other_dynamic_state();
                        if self.device.is_some() {
                            if self.last_state_check_time.elapsed().as_secs_f32() >= 3.0 {
                                if let Err(_e) = self.check_device_state_changes() {
                                    let _ = self.read_device_status();
                                }
                                self.last_state_check_time = std::time::Instant::now();
                            }
                        }
                    }
                    self.last_refresh_time = std::time::Instant::now();
                }
            }
        }
        if self.detecting_device && self.device.is_none() && self.device_detection_done {
            if std::time::Instant::now() >= self.min_detecting_until {
                self.detecting_device = false;
            }
        }
        let footer_height = egui::TopBottomPanel::bottom("footer")
            .show(ctx, |ui| {
                ui::footer::render_footer(ui, &mut self.status_messages);
            })
            .response
            .rect
            .height();
        let central_response = egui::CentralPanel::default().show(ctx, |ui| {
            ui::header::render_header(
                ui,
                ctx,
                self.loading,
                &self.system_specs,
                &self.device,
                &self.message_manager,
                self.detecting_device,
            );
            ui.separator();
            self.render_performance_section(ui);
            ui.separator();
            self.render_fan_section(ui);
            ui.separator();
            self.render_lighting_section(ui);
            ui.separator();
            self.render_battery_section(ui);
        });
        let custom_visible_now = self.device.is_some() && self.status.performance_mode == "Custom";
        if self.base_window_height == 0.0 {
            self.base_window_height =
                central_response.response.rect.height() + footer_height + 16.0;
        }
        if custom_visible_now != self.custom_controls_visible_last {
            let width = 450.0;
            if custom_visible_now {
                let added = 3.0 * ctx.style().spacing.interact_size.y;
                self.expanded_window_height = Some(self.base_window_height + added);
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    width,
                    self.expanded_window_height.unwrap(),
                )));
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    width,
                    self.base_window_height,
                )));
            }
            self.custom_controls_visible_last = custom_visible_now;
        }
    }
}
