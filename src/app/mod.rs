use crate::device::CompleteDeviceState;
use crate::messaging::{error_message, status_message, MessageManager};
use crate::system::SystemSpecs;
use eframe::egui;
use librazer::command;
use librazer::device::Device;
use librazer::types::{CpuBoost, GpuBoost, PerfMode};
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub struct DeviceStatus {
    pub performance_mode: String,
    pub fan_speed: String,
    pub fan_rpm: Option<u16>,
    pub fan_actual_rpm: Option<u16>,
    pub logo_mode: String,
    pub keyboard_brightness: u8,
    pub lights_always_on: bool,
    pub battery_care: bool,
    pub battery_threshold: u8,
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self {
            performance_mode: "Reading...".to_string(),
            fan_speed: "Reading...".to_string(),
            fan_rpm: None,
            fan_actual_rpm: None,
            logo_mode: "Reading...".to_string(),
            keyboard_brightness: 0,
            lights_always_on: false,
            battery_care: false,
            battery_threshold: 80,
        }
    }
}

#[derive(Debug, Clone)]
pub enum InitMessage {
    SystemSpecsComplete(SystemSpecs),
    PowerStateRead(bool),
    InitializationComplete,
    DeviceDetectionComplete(bool),
}

pub struct RazerGuiApp {
    pub status: DeviceStatus,
    pub device: Option<Device>,
    pub device_state: Option<CompleteDeviceState>,
    pub system_specs: SystemSpecs,
    pub available_performance_modes: Vec<PerfMode>,
    pub base_performance_modes: Vec<PerfMode>,

    pub ac_power: bool,
    pub ac_profile: CompleteDeviceState,
    pub battery_profile: CompleteDeviceState,

    pub loading: bool,
    pub fully_initialized: bool,
    pub init_receiver: Option<mpsc::Receiver<InitMessage>>,
    pub message_manager: MessageManager,
    pub last_refresh_time: std::time::Instant,
    pub last_state_check_time: std::time::Instant,
    pub last_fan_enforce_time: std::time::Instant,
    pub status_messages: bool,

    pub manual_fan_rpm: u16,
    pub temp_brightness_step: usize,
    pub brightness_slider_active: bool,
    pub battery_slider_active: bool,
    pub should_quit: bool,

    pub init_power_read: bool,
    pub init_specs_complete: bool,
    pub last_perf_poll_time: std::time::Instant,
    pub cpu_boost: CpuBoost,
    pub gpu_boost: GpuBoost,
    pub base_window_height: f32,
    pub expanded_window_height: Option<f32>,
    pub custom_controls_visible_last: bool,
    pub detecting_device: bool,
    pub device_detection_done: bool,
    pub min_detecting_until: std::time::Instant,
}

impl RazerGuiApp {
    pub fn new() -> Self {
        let ac_profile = CompleteDeviceState::default();
        let battery_profile = CompleteDeviceState {
            perf_mode: PerfMode::Battery,
            battery_threshold: 80,
            ..CompleteDeviceState::default()
        };

        let (init_sender, init_receiver) = mpsc::channel();
        let now = std::time::Instant::now();
        let mut app = Self {
            status: DeviceStatus::default(),
            device: None,
            device_state: None,
            system_specs: SystemSpecs::default(),
            available_performance_modes: Vec::new(),
            base_performance_modes: Vec::new(),
            ac_power: true,
            ac_profile,
            battery_profile,
            loading: true,
            fully_initialized: false,
            init_receiver: Some(init_receiver),
            message_manager: MessageManager::new(),
            last_refresh_time: std::time::Instant::now(),
            last_state_check_time: std::time::Instant::now(),
            last_fan_enforce_time: std::time::Instant::now(),
            status_messages: false,
            manual_fan_rpm: 2000,
            temp_brightness_step: 0,
            brightness_slider_active: false,
            battery_slider_active: false,
            should_quit: false,
            init_power_read: false,
            init_specs_complete: false,
            last_perf_poll_time: std::time::Instant::now(),
            cpu_boost: CpuBoost::Low,
            gpu_boost: GpuBoost::Low,
            base_window_height: 0.0,
            expanded_window_height: None,
            custom_controls_visible_last: false,
            detecting_device: true,
            device_detection_done: false,
            min_detecting_until: now + std::time::Duration::from_secs(1),
        };

        app.start_device_detection(init_sender.clone());
        app.start_background_initialization(init_sender);
        app
    }

    // Messaging helpers
    pub fn set_status_message(&mut self, message: String) {
        self.message_manager.add_message(status_message(message));
    }
    pub fn set_optional_status_message(&mut self, message: String) {
        if self.status_messages {
            self.message_manager.add_message(status_message(message));
        }
    }
    pub fn set_error_message(&mut self, message: String) {
        self.message_manager.add_message(error_message(message));
    }

    // Common convenience helper
    pub fn set_no_device_message(&mut self) {
        self.set_status_message("No device connected".to_string());
    }

    pub fn update_stored_device_state(&mut self) {
        if let Some(ref device) = self.device {
            if let Ok(current_state) = CompleteDeviceState::read_from_device(device) {
                self.device_state = Some(current_state);
            }
        }
    }
}

// Shared low-level helpers used by submodules
pub(crate) fn get_fan_rpm_actual(device: &Device, zone: librazer::types::FanZone) -> Option<u16> {
    match command::get_fan_actual_rpm(device, zone) {
        Ok(rpm) => Some(rpm),
        Err(_) => None,
    }
}

pub(crate) fn get_fan_rpm_set(device: &Device, zone: librazer::types::FanZone) -> Option<u16> {
    match command::get_fan_rpm(device, zone) {
        Ok(rpm) => Some(rpm),
        Err(_) => None,
    }
}

mod battery;
mod fan;
mod init;
mod lighting;
mod perf_ui;
mod performance;
mod state;

impl eframe::App for RazerGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_impl(ctx);
    }
}
