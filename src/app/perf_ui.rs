use eframe::egui;
use librazer::command;
use librazer::types::{CpuBoost, FanMode, GpuBoost};

use super::RazerGuiApp;

impl RazerGuiApp {
    pub fn set_performance_mode(&mut self, mode: &str) {
        let perf_mode = match super::RazerGuiApp::string_to_perf_mode(mode) {
            Some(m) => m,
            None => return,
        };
        let mut restore_manual = None::<u16>;
        let mut read_boosts = false;
        let mut set_mode_ok = false;
        let mut error_msg: Option<String> = None;
        if let Some(ref device) = self.device {
            let (current_fan_mode, set_rpm) = super::RazerGuiApp::read_current_fan_state(device);
            match command::set_perf_mode(device, perf_mode) {
                Ok(_) => {
                    set_mode_ok = true;
                    if matches!(current_fan_mode, FanMode::Manual) {
                        restore_manual = set_rpm;
                    }
                    if mode == "Custom" {
                        read_boosts = true;
                    }
                }
                Err(e) => {
                    error_msg = Some(format!("Failed to set performance mode: {}", e));
                }
            }
            if set_mode_ok {
                if let Some(rpm) = restore_manual {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    if command::set_fan_mode(device, FanMode::Manual).is_ok() {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        if command::set_fan_rpm(device, rpm, true).is_err() {
                            error_msg = Some(
                                "Failed to restore fan RPM after performance mode change".into(),
                            );
                        } else {
                            restore_manual = Some(rpm);
                        }
                    } else {
                        error_msg = Some(
                            "Failed to restore manual fan mode after performance mode change"
                                .into(),
                        );
                    }
                }
                if read_boosts {
                    if let Ok(v) = command::get_cpu_boost(device) {
                        self.cpu_boost = v;
                    }
                    if let Ok(v) = command::get_gpu_boost(device) {
                        self.gpu_boost = v;
                    }
                }
            }
        } else {
            self.set_no_device_message();
            return;
        }
        if let Some(msg) = error_msg {
            self.set_error_message(msg);
        }
        if set_mode_ok {
            self.status.performance_mode = mode.to_string();
            if let Some(rpm) = restore_manual {
                self.status.fan_speed = "Manual".into();
                self.status.fan_rpm = Some(rpm);
                self.manual_fan_rpm = rpm;
            }
            self.set_optional_status_message("Mode changed".into());
            self.update_stored_device_state();
        }
    }

    pub fn render_performance_section(&mut self, ui: &mut egui::Ui) {
        use crate::ui::performance::{render_performance_section, PerformanceAction};
        let (mut allowed_cpu, mut allowed_gpu, disallowed_pairs) =
            self.get_descriptor_allowed_boosts();
        let base_cpu = allowed_cpu.clone();
        let base_gpu = allowed_gpu.clone();
        let showing_hidden =
            ui.ctx().data(|d| d.get_temp::<bool>("perf_hidden_show".into()).unwrap_or(false));
        if showing_hidden {
            let full_cpu = [CpuBoost::Low, CpuBoost::Medium, CpuBoost::High, CpuBoost::Boost];
            let full_gpu = [GpuBoost::Low, GpuBoost::Medium, GpuBoost::High];
            for b in full_cpu {
                if !allowed_cpu.contains(&b) {
                    allowed_cpu.push(b);
                }
            }
            for b in full_gpu {
                if !allowed_gpu.contains(&b) {
                    allowed_gpu.push(b);
                }
            }
            let order_cpu = |b: &CpuBoost| match b {
                CpuBoost::Low => 0,
                CpuBoost::Medium => 1,
                CpuBoost::High => 2,
                CpuBoost::Boost => 3,
                CpuBoost::Undervolt => 4,
            };
            allowed_cpu.sort_by_key(order_cpu);
            let order_gpu = |b: &GpuBoost| match b {
                GpuBoost::Low => 0,
                GpuBoost::Medium => 1,
                GpuBoost::High => 2,
            };
            allowed_gpu.sort_by_key(order_gpu);
        }
        let action = render_performance_section(
            ui,
            &self.status.performance_mode,
            self.ac_power,
            &self.available_performance_modes,
            &self.base_performance_modes,
            self.status_messages,
            self.cpu_boost,
            self.gpu_boost,
            &allowed_cpu,
            &allowed_gpu,
            &disallowed_pairs,
            &base_cpu,
            &base_gpu,
            self.device.is_none(),
        );
        match action {
            PerformanceAction::None => {}
            PerformanceAction::SetPerformanceMode(mode) => {
                self.set_performance_mode(&mode);
            }
            PerformanceAction::ToggleHidden => {
                let current = ui
                    .ctx()
                    .data(|d| d.get_temp::<bool>("perf_hidden_show".into()).unwrap_or(false));
                ui.ctx().data_mut(|d| d.insert_temp("perf_hidden_show".into(), !current));
            }
            PerformanceAction::SetCpuBoost(boost) => {
                if self.status.performance_mode == "Custom" {
                    if let Some(ref device) = self.device {
                        if let Err(e) = command::set_cpu_boost(device, boost) {
                            self.set_error_message(format!("Failed CPU boost: {}", e));
                        } else {
                            self.cpu_boost = boost;
                            self.set_optional_status_message(format!("CPU {:?}", boost));
                        }
                    }
                }
            }
            PerformanceAction::SetGpuBoost(boost) => {
                if self.status.performance_mode == "Custom" {
                    if let Some(ref device) = self.device {
                        if let Err(e) = command::set_gpu_boost(device, boost) {
                            self.set_error_message(format!("Failed GPU boost: {}", e));
                        } else {
                            self.gpu_boost = boost;
                            self.set_optional_status_message(format!("GPU {:?}", boost));
                        }
                    }
                }
            }
        }
    }
}
