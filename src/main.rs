#![windows_subsystem = "windows"]

mod power;
mod device;
mod ui;
mod system;
mod messaging;
mod utils;

use eframe::egui;
use egui::IconData;

use anyhow::Result;
use std::sync::mpsc;

use librazer::types::{BatteryCare, LightsAlwaysOn, LogoMode, FanMode, PerfMode};
use librazer::{command, device::Device};
use strum::IntoEnumIterator;

use power::get_power_state;
use device::CompleteDeviceState;
use system::{SystemSpecs, get_system_specs};
use messaging::{MessageManager, error_message, status_message};
use utils::{execute_device_command_simple, DeviceStateReader};

#[derive(Debug, Clone)]
enum InitMessage {
    SystemSpecsComplete(SystemSpecs),
    PowerStateRead(bool),
    InitializationComplete,
}

#[derive(Debug, Clone)]
struct DeviceStatus {
    performance_mode: String,
    fan_speed: String,
    fan_rpm: Option<u16>,
    fan_actual_rpm: Option<u16>,
    logo_mode: String,
    keyboard_brightness: u8,
    lights_always_on: bool,
    battery_care: bool,
    // battery_threshold removed (not used)
}

impl Default for DeviceStatus {
    fn default() -> Self {
        Self {
            performance_mode: "Reading...".to_string(),
            fan_speed: "Reading...".to_string(),
            fan_rpm: None,
            fan_actual_rpm: None,
            logo_mode: "Reading...".to_string(),
            keyboard_brightness: 0, // Will be read from device immediately
            lights_always_on: false,
            battery_care: true,
            
        }
    }
}

struct RazerGuiApp {
    status: DeviceStatus,
    device: Option<Device>,
    device_state: Option<CompleteDeviceState>,
    system_specs: SystemSpecs,
    available_performance_modes: Vec<PerfMode>, // Dynamically detected available modes
    
    ac_power: bool,
    ac_profile: CompleteDeviceState,
    battery_profile: CompleteDeviceState,
    
    loading: bool,
    fully_initialized: bool,
    init_receiver: Option<mpsc::Receiver<InitMessage>>, // Channel for background initialization
    message_manager: MessageManager,
    last_refresh_time: std::time::Instant,
    last_state_check_time: std::time::Instant,
    last_fan_enforce_time: std::time::Instant, // Track last manual fan RPM enforcement
    status_messages: bool,
    // settings window removed
    
    manual_fan_rpm: u16,
    temp_brightness_step: usize, // 0-15 step index for discrete brightness levels
    brightness_slider_active: bool, // Track if user is actively using brightness slider
    should_quit: bool,

    // Initialization progress flags
    init_power_read: bool,
    init_specs_complete: bool,
    last_perf_poll_time: std::time::Instant,
    // Async perf-mode probe receiver
    probe_receiver: Option<mpsc::Receiver<Vec<PerfMode>>>,
}

impl RazerGuiApp {
    // Initialization
    
    /// Convert PerfMode enum to display string (uses Debug name to be future-proof)
    fn perf_mode_to_string(mode: PerfMode) -> String {
        format!("{:?}", mode)
    }
    
    /// Convert string to PerfMode enum by matching Debug names (future-proof)
    fn string_to_perf_mode(mode: &str) -> Option<PerfMode> {
        PerfMode::iter().find(|m| format!("{:?}", m) == mode)
    }
    
    /// Convert LogoMode enum to display string
    fn logo_mode_to_string(mode: LogoMode) -> &'static str {
        match mode {
            LogoMode::Static => "Static",
            LogoMode::Breathing => "Breathing",
            LogoMode::Off => "Off",
        }
    }
    
    /// Convert string to LogoMode enum
    fn string_to_logo_mode(mode: &str) -> Option<LogoMode> {
        match mode {
            "Static" => Some(LogoMode::Static),
            "Breathing" => Some(LogoMode::Breathing),
            "Off" => Some(LogoMode::Off),
            _ => None,
        }
    }
    
    // Read current fan state from device
    fn read_current_fan_state(device: &Device) -> (FanMode, Option<u16>) {
        // Read the actual fan mode from device with retry
        let fan_mode = if let Ok((_, fan_mode)) = command::get_perf_mode(device) {
            fan_mode
        } else {
            // Retry once if the first attempt failed
            if let Ok((_, fan_mode)) = command::get_perf_mode(device) {
                fan_mode
            } else {
                eprintln!("Warning: Failed to read device fan mode, unable to determine current state");
                FanMode::Auto
            }
        };
        
        // Read the SET RPM (what user configured), not the actual RPM
        let set_rpm = get_fan_rpm_set(device, librazer::types::FanZone::Zone1);
        
        (fan_mode, set_rpm)
    }

    // Convert fan mode to UI values
    fn get_fan_status_from_mode(fan_mode: FanMode, device: &Device) -> (String, Option<u16>) {
        match fan_mode {
            FanMode::Auto => ("Auto".to_string(), None),
            FanMode::Manual => {
                let set_rpm = get_fan_rpm_set(device, librazer::types::FanZone::Zone1);
                ("Manual".to_string(), set_rpm)
            }
        }
    }

    fn set_no_device_message(&mut self) {
        self.set_status_message("No device connected".to_string());
    }
    
    fn new() -> Self {
        let ac_profile = CompleteDeviceState::default();
        let battery_profile = CompleteDeviceState {
            perf_mode: PerfMode::Battery,
            ..CompleteDeviceState::default()
        };
        
        let (init_sender, init_receiver) = mpsc::channel();
        
        let mut app = Self {
            status: DeviceStatus::default(),
            device: None,
            device_state: None,
            system_specs: SystemSpecs::default(),
            available_performance_modes: Vec::new(),
            ac_power: true,
            ac_profile,
            battery_profile,
            loading: true, // Start in loading state
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
            
            should_quit: false,

            init_power_read: false,
            init_specs_complete: false,
            last_perf_poll_time: std::time::Instant::now(),
            probe_receiver: None,
        };
        
        app.init_device();
        
        app.read_initial_device_state();
        
        app.start_background_initialization(init_sender);
        
        app
    }

    fn init_device(&mut self) {
        match Device::detect() {
            Ok(dev) => {
                self.device = Some(dev);
            }
            Err(e) => {
                self.set_error_message(format!("Failed to connect to Razer device: {}", e));
            }
        }
        
        self.detect_available_performance_modes();
    }

    fn detect_available_performance_modes(&mut self) {
        // Prefer descriptor-provided list; else show all
        if let Some(ref device) = self.device {
            if let Some(list) = device.info().perf_modes {
                self.available_performance_modes = list.to_vec();
                return;
            }
        }
        self.available_performance_modes = PerfMode::iter().collect();
    }
    
    fn read_initial_device_state(&mut self) {
        if let Some(ref device) = self.device {
            let mut reader = DeviceStateReader::new(device);
            
            if let Some(brightness) = reader.read(|d| command::get_keyboard_brightness(d), "keyboard brightness") {
                self.status.keyboard_brightness = brightness;
                self.temp_brightness_step = ui::lighting::raw_brightness_to_step_index(brightness);
            }
            
            if let Some((perf_mode, fan_mode)) = reader.read(|d| command::get_perf_mode(d), "performance mode") {
                self.status.performance_mode = Self::perf_mode_to_string(perf_mode).to_string();
                
                let (fan_speed, fan_rpm) = Self::get_fan_status_from_mode(fan_mode, device);
                self.status.fan_speed = fan_speed;
                self.status.fan_rpm = fan_rpm;
                
                if let Some(rpm) = fan_rpm {
                    self.manual_fan_rpm = rpm;
                }
            }
            
            if self.status.fan_speed == "Reading..." {
                if let Ok((_, fan_mode)) = command::get_perf_mode(device) {
                    let (fan_speed, fan_rpm) = Self::get_fan_status_from_mode(fan_mode, device);
                    self.status.fan_speed = fan_speed;
                    self.status.fan_rpm = fan_rpm;
                    
                    if let Some(rpm) = fan_rpm {
                        self.manual_fan_rpm = rpm;
                    }
                }
            }
            
            if let Some(lights_always_on) = reader.read(|d| command::get_lights_always_on(d), "lights always on") {
                self.status.lights_always_on = matches!(lights_always_on, LightsAlwaysOn::Enable);
            }
            
            if let Some(battery_care) = reader.read(|d| command::get_battery_care(d), "battery care") {
                self.status.battery_care = matches!(battery_care, BatteryCare::Enable);
            }
            
            let errors = reader.finish();
            if !errors.is_empty() && cfg!(debug_assertions) {
                eprintln!("Device state reading errors: {:?}", errors);
            }
        }
    }
    
    fn start_background_initialization(&mut self, sender: mpsc::Sender<InitMessage>) {
        let device_name = if let Some(ref device) = self.device {
            Some(device.info().name.to_string())
        } else {
            None
        };

        std::thread::spawn(move || {
            if let Ok(ac_power) = get_power_state() {
                let _ = sender.send(InitMessage::PowerStateRead(ac_power));
            }

            // Mark initialization complete early to let UI proceed; specs will arrive later
            let _ = sender.send(InitMessage::InitializationComplete);

            // Load system specs afterwards (slower) and send when ready
            let device_name_ref = device_name.as_deref();
            let system_specs = get_system_specs(device_name_ref);
            let _ = sender.send(InitMessage::SystemSpecsComplete(system_specs));
        });

        self.loading = false;

        self.set_status_message("Initializing...".to_string());
    }
    
    fn process_background_initialization(&mut self) {
        let mut messages_to_process = Vec::new();
        
        if let Some(ref receiver) = self.init_receiver {
            while let Ok(message) = receiver.try_recv() {
                messages_to_process.push(message);
            }
        }
        
        for message in messages_to_process {
            match message {
                InitMessage::SystemSpecsComplete(specs) => {
                    self.system_specs = specs;
                    self.init_specs_complete = true;
                    // Only show complete when all background init pieces are done
                    if self.fully_initialized && self.init_power_read && self.init_specs_complete {
                        self.set_status_message("Initialization complete".to_string());
                    } else {
                        self.set_optional_status_message("System specifications loaded".to_string());
                    }
                }
                InitMessage::PowerStateRead(ac_power) => {
                    self.ac_power = ac_power;
                    self.init_power_read = true;
                    // Don't show message for initial power state
                }
                InitMessage::InitializationComplete => {
                    self.fully_initialized = true;
                    // Do not show completion yet; wait for specs as well
                    // Read device status from main thread since we can't clone Device
                    if self.device.is_some() {
                        if let Err(e) = self.read_device_status() {
                            self.set_error_message(format!("Failed to read device status: {}", e));
                        } else {
                            self.update_stored_device_state();
                            self.sync_ui_with_device_state();
                            self.init_fan_slider_from_device();
                            // Start non-blocking perf mode probing if list unknown and not Custom
                            self.maybe_probe_perf_modes_async();
                        }
                    }
                }
            }
        }
    }

    fn maybe_probe_perf_modes_async(&mut self) {
        self.start_probe_perf_modes(false);
    }

    fn start_probe_perf_modes(&mut self, force: bool) {
        // Only when device supported and descriptor didn’t specify modes
        let Some(ref device) = self.device else { return; };
        if !force && device.info().perf_modes.is_some() { return; }
        // Don’t probe if current mode is Custom
        if let Ok((perf_mode, _)) = command::get_perf_mode(device) {
            if matches!(perf_mode, PerfMode::Custom) { return; }
        }
        // Spawn a thread to probe; clone minimal info by device handle is not clonable, so use a weak pattern via channel
        // Use a simple thread to probe and then set available_performance_modes on main thread via a queued closure
        // Here, we’ll instead store results in a static parking_lot since egui main thread only; keep it simple: use a channel
        let (res_tx, res_rx) = std::sync::mpsc::channel::<Vec<PerfMode>>();
        std::thread::spawn({
            // We cannot move &Device across thread safely; reopen via detect for probing
            move || {
                let mut modes = Vec::new();
                // Re-detect to open a fresh handle
                if let Ok(dev) = Device::detect() {
                    if let Ok((current_mode, _)) = command::get_perf_mode(&dev) {
                        for m in PerfMode::iter() {
                            if m == current_mode { modes.push(m); continue; }
                            if command::set_perf_mode(&dev, m).is_ok() {
                                modes.push(m);
                            }
                        }
                        // try to restore
                        let _ = command::set_perf_mode(&dev, current_mode);
                    }
                }
                let _ = res_tx.send(modes);
            }
        });

        // Poll result on future updates
        // Store receiver to check later
        self.probe_receiver = Some(res_rx);
    }
}

/// Get fan ACTUAL RPM using librazer for live monitoring (not the SET RPM)
fn get_fan_rpm_actual(device: &Device, zone: librazer::types::FanZone) -> Option<u16> {
    // Use librazer for reliable ACTUAL fan RPM readings (live monitoring)
    match command::get_fan_actual_rpm(device, zone) {
        Ok(rpm) => Some(rpm),
        Err(_) => None,
    }
}

/// Get fan SET RPM using librazer (what the user configured)
fn get_fan_rpm_set(device: &Device, zone: librazer::types::FanZone) -> Option<u16> {
    // Use librazer to read the SET RPM value (user configuration)
    match command::get_fan_rpm(device, zone) {
        Ok(rpm) => Some(rpm),
        Err(_) => None,
    }
}

impl RazerGuiApp {

    // ========================================================================
    // Device Control Methods
    // ========================================================================

    fn read_device_status(&mut self) -> Result<()> {
        let device = self.device.as_ref().unwrap(); // We know it exists from the caller
        // Read performance mode
        let (perf_mode, fan_mode) = command::get_perf_mode(device)?;
        self.status.performance_mode = Self::perf_mode_to_string(perf_mode).to_string();

        // Read fan status using new method
        let (fan_speed, fan_rpm) = Self::get_fan_status_from_mode(fan_mode, device);
        self.status.fan_speed = fan_speed;
        self.status.fan_rpm = fan_rpm;
        if let Some(rpm) = fan_rpm {
            self.manual_fan_rpm = rpm;
        }

        // Read actual fan RPM for live readout using librazer
        self.status.fan_actual_rpm = get_fan_rpm_actual(device, librazer::types::FanZone::Zone1);

        // Read lighting status
        if let Ok(logo_mode) = command::get_logo_mode(device) {
            self.status.logo_mode = Self::logo_mode_to_string(logo_mode).to_string();
        }

        if let Ok(brightness) = command::get_keyboard_brightness(device) {
            self.status.keyboard_brightness = brightness;
            // Always update display on startup/refresh (brightness slider not active yet)
            self.temp_brightness_step = ui::lighting::raw_brightness_to_step_index(brightness);
        }

        if let Ok(lights_always_on) = command::get_lights_always_on(device) {
            self.status.lights_always_on = matches!(lights_always_on, LightsAlwaysOn::Enable);
        }

        // Read battery care status
        if let Ok(battery_care) = command::get_battery_care(device) {
            self.status.battery_care = matches!(battery_care, BatteryCare::Enable);
        }

        Ok(())
    }

    fn sync_ui_with_device_state(&mut self) {
        // Sync UI with current device state without full device communication
        if let Some(ref device) = self.device {
            // Update brightness display if user is not actively using the slider
            if !self.brightness_slider_active {
                if let Ok(brightness) = command::get_keyboard_brightness(device) {
                    self.status.keyboard_brightness = brightness;
                    self.temp_brightness_step = ui::lighting::raw_brightness_to_step_index(brightness);
                }
            }
            
            // Sync fan settings from device (important for external changes)
            let (fan_mode, set_rpm) = Self::read_current_fan_state(device);
            let (fan_speed, fan_rpm) = Self::get_fan_status_from_mode(fan_mode, device);
            self.status.fan_speed = fan_speed;
            self.status.fan_rpm = fan_rpm;
            
            // Update manual fan RPM slider to match device SET RPM setting
            if let Some(rpm) = set_rpm {
                self.manual_fan_rpm = rpm;
            }
            
            // Update toggle states
            if let Ok(lights_always_on) = command::get_lights_always_on(device) {
                self.status.lights_always_on = matches!(lights_always_on, LightsAlwaysOn::Enable);
            }
            
            if let Ok(battery_care) = command::get_battery_care(device) {
                self.status.battery_care = matches!(battery_care, BatteryCare::Enable);
            }
        }
    }

    fn sync_other_dynamic_state(&mut self) {
        // Only sync toggle states (brightness is handled separately at higher frequency)
        if let Some(ref device) = self.device {
            // Update toggle states (can change via external tools)
            if let Ok(lights_always_on) = command::get_lights_always_on(device) {
                self.status.lights_always_on = matches!(lights_always_on, LightsAlwaysOn::Enable);
            }
            
            if let Ok(battery_care) = command::get_battery_care(device) {
                self.status.battery_care = matches!(battery_care, BatteryCare::Enable);
            }
        }
    }

    fn read_essential_device_state(&mut self) -> Result<()> {
        // Read only essential dynamic state (performance mode, fan mode) - no static data
        let device = self.device.as_ref().unwrap();
        
        // Read performance mode (can change via external tools)
        let (perf_mode, fan_mode) = command::get_perf_mode(device)?;
        self.status.performance_mode = Self::perf_mode_to_string(perf_mode).to_string();

        // Read fan status using new method (can change via external tools)
        let (fan_speed, fan_rpm) = Self::get_fan_status_from_mode(fan_mode, device);
        self.status.fan_speed = fan_speed;
        self.status.fan_rpm = fan_rpm;
        if let Some(rpm) = fan_rpm {
            self.manual_fan_rpm = rpm;
        }

        // Logo mode (can change via external tools)
        if let Ok(logo_mode) = command::get_logo_mode(device) {
            self.status.logo_mode = Self::logo_mode_to_string(logo_mode).to_string();
        }

        Ok(())
    }

    fn init_fan_slider_from_device(&mut self) {
        // Initialize fan slider with current device setting on startup
        if let Some(ref device) = self.device {
            let (fan_mode, set_rpm) = Self::read_current_fan_state(device);
            
            // Update UI to reflect current device state
            let (fan_speed, fan_rpm) = Self::get_fan_status_from_mode(fan_mode, device);
            self.status.fan_speed = fan_speed;
            self.status.fan_rpm = fan_rpm;
            
            // Set manual fan RPM slider to current device SET RPM setting
            if let Some(rpm) = set_rpm {
                self.manual_fan_rpm = rpm;
            }
        }
    }

    fn check_device_state_changes(&mut self) -> Result<()> {
        if let Some(ref device) = self.device {
            let current_state = CompleteDeviceState::read_from_device(device)?;
            
            if let Some(ref stored_state) = self.device_state {
                if current_state != *stored_state {
                    // State changed externally! Update our stored state and UI
                    let old_perf_mode = Self::perf_mode_to_string(stored_state.perf_mode);
                    let new_perf_mode = Self::perf_mode_to_string(current_state.perf_mode);
                    
                    self.device_state = Some(current_state.clone());
                    
                    // Convert the low-level state to our UI format
                    self.status.performance_mode = Self::perf_mode_to_string(current_state.perf_mode).to_string();
                    
                    let (fan_speed, fan_rpm) = Self::get_fan_status_from_mode(current_state.fan_mode, device);
                    self.status.fan_speed = fan_speed;
                    self.status.fan_rpm = fan_rpm;
                    if let Some(rpm) = fan_rpm {
                        self.manual_fan_rpm = rpm;
                    }
                    
                    self.status.logo_mode = Self::logo_mode_to_string(current_state.logo_mode).to_string();
                    
                    self.status.keyboard_brightness = current_state.keyboard_brightness;
                    self.temp_brightness_step = ui::lighting::raw_brightness_to_step_index(current_state.keyboard_brightness);
                    
                    self.status.lights_always_on = matches!(current_state.lights_always_on, LightsAlwaysOn::Enable);
                    self.status.battery_care = matches!(current_state.battery_care, BatteryCare::Enable);
                    
                    // Show specific change message
                    if old_perf_mode != new_perf_mode {
                        self.set_status_message(format!("Performance mode changed externally: {} → {}", old_perf_mode, new_perf_mode));
                    } else if self.status_messages {
                        self.set_optional_status_message("Device state updated externally".to_string());
                    }
                }
            } else {
                // First time - just store the current state
                self.device_state = Some(current_state);
            }
        }
        Ok(())
    }

    fn set_status_message(&mut self, message: String) {
        self.message_manager.add_message(status_message(message));
    }

    fn set_optional_status_message(&mut self, message: String) {
        if self.status_messages {
            self.message_manager.add_message(status_message(message));
        }
    }

    fn set_error_message(&mut self, message: String) {
        self.message_manager.add_message(error_message(message));
    }

    fn clear_status_message_if_disabled(&mut self) {
        // Clear any existing optional status messages if status messages are disabled
        // Important status messages (like device connections, mode changes) still show
        if !self.status_messages {
            // Only clear messages that are truly optional
            // For now, we'll leave this empty since we changed most messages to be important
        }
    }

    fn update_stored_device_state(&mut self) {
        // After making a change, update our stored state to match current device state
        if let Some(ref device) = self.device {
            if let Ok(current_state) = CompleteDeviceState::read_from_device(device) {
                self.device_state = Some(current_state);
            }
        }
    }

    fn auto_switch_profile(&mut self) {
        if let Some(ref device) = self.device {
            let target_profile = if self.ac_power {
                self.ac_profile.clone()
            } else {
                self.battery_profile.clone()
            };
            
            let profile_name = if self.ac_power { "AC" } else { "Battery" };
            
            // Only apply performance mode
            if let Err(e) = command::set_perf_mode(device, target_profile.perf_mode) {
                self.set_error_message(format!("Failed to switch to {} profile: {}", profile_name, e));
                return;
            }
            
            // Update performance mode in UI
            self.status.performance_mode = Self::perf_mode_to_string(target_profile.perf_mode).to_string();
            
            self.set_status_message(format!("⚡ Auto-switched to {} profile", profile_name));
        }
        
        // Read current device state to preserve user settings
        if let Err(_) = self.read_device_status() {
            // If we can't read device status, try to apply minimal fallback
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
        
        // Update stored state
        self.update_stored_device_state();
        
        // Sync UI with current device state
        self.sync_ui_with_device_state();
    }

    fn apply_profile(&self, device: &Device, profile: &CompleteDeviceState) -> Result<()> {
        command::set_perf_mode(device, profile.perf_mode)?;
        
        command::set_logo_mode(device, profile.logo_mode)?;
        
        // Apply keyboard brightness if different from current
        if let Ok(current_brightness) = command::get_keyboard_brightness(device) {
            if current_brightness != profile.keyboard_brightness {
                command::set_keyboard_brightness(device, profile.keyboard_brightness)?;
            }
        } else {
            command::set_keyboard_brightness(device, profile.keyboard_brightness)?;
        }
        
        command::set_lights_always_on(device, profile.lights_always_on)?;
        
        // Apply battery care
        command::set_battery_care(device, profile.battery_care)?;
        
        Ok(())
    }

    fn set_performance_mode(&mut self, mode: &str) {
        let perf_mode = match Self::string_to_perf_mode(mode) {
            Some(mode) => mode,
            None => return,
        };
        
        if let Some(ref device) = self.device {
            // Read current fan settings before changing performance mode
            let (current_fan_mode, set_rpm) = Self::read_current_fan_state(device);
            
            match command::set_perf_mode(device, perf_mode) {
                Ok(_) => {
                    self.status.performance_mode = mode.to_string();
                    
                    // IMPORTANT: set_perf_mode() internally sets fan mode to Auto!
                    // We need to restore the original fan settings with a small delay
                    if matches!(current_fan_mode, FanMode::Manual) {
                        if let Some(rpm) = set_rpm {
                            // Small delay to let the performance mode change settle
                            std::thread::sleep(std::time::Duration::from_millis(50));
                            
                            // Restore manual mode first, then set RPM
                            if let Ok(_) = command::set_fan_mode(device, FanMode::Manual) {
                                // Another small delay before setting RPM
                                std::thread::sleep(std::time::Duration::from_millis(50));
                                
                                if let Ok(_) = command::set_fan_rpm(device, rpm, true) {
                                    self.status.fan_speed = "Manual".to_string();
                                    self.status.fan_rpm = Some(rpm);
                                    self.manual_fan_rpm = rpm;
                                } else {
                                    self.set_error_message("Failed to restore fan RPM after performance mode change".to_string());
                                }
                            } else {
                                self.set_error_message("Failed to restore manual fan mode after performance mode change".to_string());
                            }
                        }
                    }
                    // Note: Auto mode doesn't need restoration since set_perf_mode already sets it to Auto
                    
                    self.set_optional_status_message(format!("Performance mode set to {}", mode));
                },
                Err(e) => {
                    self.set_error_message(format!("Failed to set performance mode: {}", e));
                }
            }
            self.update_stored_device_state();
        } else {
            self.set_no_device_message();
        }
    }

    fn render_performance_section(&mut self, ui: &mut egui::Ui) {
        use ui::performance::{render_performance_section, PerformanceAction};
        
        let action = render_performance_section(
            ui,
            &self.status.performance_mode,
            self.ac_power,
            &self.system_specs.device_model,
            &self.system_specs.gpu_models,
            &self.available_performance_modes,
            self.status_messages,
        );
        
        match action {
            PerformanceAction::None => {},
            PerformanceAction::SetPerformanceMode(mode) => {
                self.set_performance_mode(&mode);
            },
            PerformanceAction::RefreshProbe => {
                self.start_probe_perf_modes(true);
            }
        }
    }

    // ========================================================================
    // GPU Mode Management - Apple-Style Clean Architecture
    // ========================================================================
    
    // GPU switching/auto GPU handling removed

    fn set_fan_mode(&mut self, mode: &str, rpm: Option<u16>) {
        if let Some(ref device) = self.device {
            let result = match mode {
                "auto" => {
                    match command::set_fan_mode(device, FanMode::Auto) {
                        Ok(_) => {
                            self.status.fan_speed = "Auto".to_string();
                            self.status.fan_rpm = None;
                            Ok(())
                        },
                        Err(e) => Err(e),
                    }
                },
                "manual" => {
                    // Set manual mode first
                    match command::set_fan_mode(device, FanMode::Manual) {
                        Ok(_) => {
                            // Then set RPM
                            let rpm_val = rpm.unwrap_or(2000);
                            match command::set_fan_rpm(device, rpm_val, true) {
                                Ok(_) => {
                                    self.status.fan_speed = "Manual".to_string();
                                    self.status.fan_rpm = Some(rpm_val);
                                    Ok(())
                                },
                                Err(e) => Err(e),
                            }
                        },
                        Err(e) => Err(e),
                    }
                }
                _ => return,
            };

            match result {
                Ok(_) => {
                    self.set_optional_status_message(format!("Fan set to {} mode", mode));
                },
                Err(e) => {
                    self.set_status_message(format!("Failed to set fan: {}", e));
                }
            }
        } else {
            self.set_no_device_message();
        }
    }

    fn set_fan_rpm_only(&mut self, rpm: u16) {
        match execute_device_command_simple(
            self.device.as_ref(),
            |device| command::set_fan_rpm(device, rpm, true),
            &format!("Fans RPM set to: {}", rpm),
            "Failed to set fan RPM"
        ) {
            Ok(message) => {
                self.status.fan_rpm = Some(rpm);
                self.set_optional_status_message(message);
            },
            Err(message) => {
                self.set_error_message(message);
            }
        }
    }

    fn enforce_manual_fan_rpm(&mut self) {
        // Silently enforce manual fan RPM by reading current SET RPM and writing it back
        // This prevents drift while respecting external app changes to the SET RPM value
        if self.status.fan_speed == "Manual" {
            if let Some(ref device) = self.device {
                // Read current SET RPM from device
                if let Some(current_set_rpm) = get_fan_rpm_set(device, librazer::types::FanZone::Zone1) {
                    // Write the same SET RPM back to prevent drift
                    if let Ok(_) = command::set_fan_rpm(device, current_set_rpm, true) {
                        // Successfully enforced - update our UI to match device
                        self.manual_fan_rpm = current_set_rpm;
                        self.status.fan_rpm = Some(current_set_rpm);
                        self.last_fan_enforce_time = std::time::Instant::now();
                    }
                }
                // Silently ignore errors to avoid spam during periodic enforcement
            }
        }
    }

    // GPU UI section removed

    fn render_fan_section(&mut self, ui: &mut egui::Ui) {
        use ui::fan::{render_fan_section, FanAction};
        
        let action = render_fan_section(
            ui,
            &self.status.fan_speed,
            self.status.fan_actual_rpm,
            self.status.fan_rpm,
            &mut self.manual_fan_rpm,
            self.status_messages,
        );
        
        match action {
            FanAction::None => {},
            FanAction::SetAutoMode => {
                self.set_fan_mode("auto", None);
            },
            FanAction::SetManualMode(rpm) => {
                self.set_fan_mode("manual", Some(rpm));
            },
            FanAction::SetManualRpm(rpm) => {
                self.set_fan_rpm_only(rpm);
            },
            FanAction::SliderDragging(_) => {
                // User is actively dragging the slider
            },
        }
    }

    fn set_logo_mode(&mut self, mode: &str) {
        let logo_mode = match Self::string_to_logo_mode(mode) {
            Some(mode) => mode,
            None => return,
        };
        
        match execute_device_command_simple(
            self.device.as_ref(),
            |device| command::set_logo_mode(device, logo_mode),
            &format!("Logo mode set to {}", mode),
            "Failed to set logo mode"
        ) {
            Ok(message) => {
                self.status.logo_mode = mode.to_string();
                self.set_optional_status_message(message);
            },
            Err(message) => {
                self.set_error_message(message);
            }
        }
    }

    fn set_brightness(&mut self, brightness: u8) {
        match execute_device_command_simple(
            self.device.as_ref(),
            |device| command::set_keyboard_brightness(device, brightness),
            &format!("Brightness set to step {}", ui::lighting::raw_brightness_to_step_index(brightness)),
            "Failed to set brightness"
        ) {
            Ok(message) => {
                self.status.keyboard_brightness = brightness;
                self.temp_brightness_step = ui::lighting::raw_brightness_to_step_index(brightness);
                self.set_optional_status_message(message);
            },
            Err(message) => {
                self.set_error_message(message);
            }
        }
    }

    fn toggle_lights_always_on(&mut self) {
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
                    // Update stored device state
                    self.update_stored_device_state();
                },
                Err(e) => {
                    self.set_status_message(format!("Failed to set lights always on: {}", e));
                    // Revert the UI change on error
                    self.status.lights_always_on = !self.status.lights_always_on;
                }
            }
        } else {
            self.set_no_device_message();
        }
    }

    fn render_lighting_section(&mut self, ui: &mut egui::Ui) {
        use ui::lighting::render_lighting_section;
        
        let action = render_lighting_section(
            ui,
            &self.status.logo_mode,
            &mut self.temp_brightness_step,
            &mut self.status.lights_always_on,
        );
        
        // Handle slider active state tracking
        if let Some(active) = action.slider_active {
            self.brightness_slider_active = active;
        }
        
        // Handle logo mode changes
        if let Some(mode) = action.logo_mode {
            self.set_logo_mode(&mode);
        }
        
        // Handle brightness changes
        if let Some(brightness) = action.brightness {
            self.set_brightness(brightness);
        }
        
        // Handle lights always on toggle
        if action.lights_always_on {
            self.toggle_lights_always_on();
        }
    }

    fn toggle_battery_care(&mut self) {
        let battery_care = if self.status.battery_care { 
            BatteryCare::Enable 
        } else { 
            BatteryCare::Disable 
        };
        
        if let Some(ref device) = self.device {
            match command::set_battery_care(device, battery_care) {
                Ok(_) => {
                    self.set_optional_status_message(format!(
                        "Battery care {}", 
                        if self.status.battery_care { "enabled" } else { "disabled" }
                    ));
                },
                Err(e) => {
                    self.set_status_message(format!("Failed to set battery care: {}", e));
                    // Revert the UI change on error
                    self.status.battery_care = !self.status.battery_care;
                }
            }
        } else {
            self.set_no_device_message();
        }
    }

    fn render_battery_section(&mut self, ui: &mut egui::Ui) {
        use ui::battery::{render_battery_section, BatteryAction};
        
    let action = render_battery_section(ui, &mut self.status.battery_care);
        
        match action {
            BatteryAction::None => {},
            BatteryAction::ToggleBatteryCare => {
                self.toggle_battery_care();
            },
        }
    }

    // settings window removed

    // (Dynamic window sizing helpers removed as GPU section is gone)
}

impl eframe::App for RazerGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // CRITICAL: Always request frequent repaints to keep update() running even when minimized
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        
        // Process background initialization messages
        self.process_background_initialization();

        // Check async perf-mode probe results
        if let Some(rx) = &self.probe_receiver {
            if let Ok(modes) = rx.try_recv() {
                if !modes.is_empty() {
                    self.available_performance_modes = modes;
                    self.set_optional_status_message("Detected supported performance modes".to_string());
                }
                self.probe_receiver = None;
            }
        }
        
        // Update message manager
        self.message_manager.update();
        
    // GPU section removed; no dynamic size based on GPU
        
    // Focus-regain refresh removed; periodic polling handles updates
        
        // When minimized, poll infrequently to catch external performance mode changes
        let minimized = ctx.input(|i| i.viewport().minimized.unwrap_or(false));
    if minimized && self.fully_initialized {
            const PERF_POLL_INTERVAL: f32 = 2.5; // seconds
            if self.last_perf_poll_time.elapsed().as_secs_f32() >= PERF_POLL_INTERVAL {
                if let Some(ref device) = self.device {
                    if let Ok((perf_mode, fan_mode)) = command::get_perf_mode(device) {
                        let new_mode = Self::perf_mode_to_string(perf_mode).to_string();
                        if self.status.performance_mode != new_mode {
                            self.status.performance_mode = new_mode;
                            let (fan_speed, fan_rpm) = Self::get_fan_status_from_mode(fan_mode, device);
                            self.status.fan_speed = fan_speed;
                            self.status.fan_rpm = fan_rpm;
            }
                    }
                }
                self.last_perf_poll_time = std::time::Instant::now();
            }
        }
        
        // Handle close request from X button
        if ctx.input(|i| i.viewport().close_requested()) {
            self.should_quit = true;
        }

        // Handle quit
        if self.should_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }

        // Continue with normal UI rendering

        self.clear_status_message_if_disabled();

    // GPU switching removed
        
        // Only update when window is not minimized to save resources
        if !ctx.input(|i| i.viewport().minimized.unwrap_or(false)) {
            // Only do regular updates if fully initialized to avoid slow operations during startup
            if self.fully_initialized {
                // Auto-refresh device status based on backlight setting
                const AUTO_REFRESH_INTERVAL: f32 = 0.5;
                if self.last_refresh_time.elapsed().as_secs_f32() >= AUTO_REFRESH_INTERVAL {
                    if self.device.is_some() && !self.loading {
                        // Check power state for responsive power switching (high frequency - 500ms)
                        if let Ok(current_ac_power) = get_power_state() {
                            if current_ac_power != self.ac_power {
                                self.ac_power = current_ac_power;
                                self.auto_switch_profile();
                                
                                // GPU auto switching removed
                            }
                        }
                    
                        // Update live fan RPM (high frequency - 500ms for responsive monitoring)
                        if let Some(ref device) = self.device {
                            self.status.fan_actual_rpm = get_fan_rpm_actual(device, librazer::types::FanZone::Zone1);
                            
                            // Update current fan mode display to show actual device state
                            let (current_fan_mode, _) = Self::read_current_fan_state(device);
                            let (fan_speed, _) = Self::get_fan_status_from_mode(current_fan_mode, device);
                            self.status.fan_speed = fan_speed; // This updates the "Current: Auto/Manual" display
                        }
                        
                        // Enforce manual fan RPM every 1 second to prevent drift
                        if self.last_fan_enforce_time.elapsed().as_secs_f32() >= 1.0 {
                            self.enforce_manual_fan_rpm();
                        }
                        
                        // Update keyboard brightness (high frequency - 500ms, can change via hardware keys)
                        if let Some(ref device) = self.device {
                            if !self.brightness_slider_active {
                                if let Ok(brightness) = command::get_keyboard_brightness(device) {
                                    self.status.keyboard_brightness = brightness;
                                    self.temp_brightness_step = ui::lighting::raw_brightness_to_step_index(brightness);
                                }
                            }
                        }
                        
                        // Only poll device for other settings when lights are always on
                        if self.status.lights_always_on {
                            // Sync other dynamic UI state (toggles) - medium frequency
                            self.sync_other_dynamic_state();
                            
                            // Check for external state changes (lower frequency - every 3 seconds)
                            if self.last_state_check_time.elapsed().as_secs_f32() >= 3.0 {
                                if let Err(_e) = self.check_device_state_changes() {
                                    // If state check fails, fall back to reading essential dynamic state
                                    if let Err(_e) = self.read_essential_device_state() {
                                        // Silently fail on auto-refresh errors to avoid spam
                                    }
                                }
                                self.last_state_check_time = std::time::Instant::now();
                            }
                        }
                    }
                    
                    self.last_refresh_time = std::time::Instant::now();
                }
            } // Close fully_initialized check
        } // Close minimize check
        
        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui::footer::render_footer(ui, &mut self.status_messages);
        });
        
        egui::CentralPanel::default().show(ctx, |ui| {
            // Header with device name and status messages
            ui::header::render_header(
                ui, 
                ctx,
                self.loading, 
                &self.system_specs,
                &self.device,
                &self.message_manager
            );
            ui.separator();

            // Performance Section
            self.render_performance_section(ui);
            ui.separator();

            // GPU Section removed

            // Fan Section
            self.render_fan_section(ui);
            ui.separator();

            // Lighting Section
            self.render_lighting_section(ui);
            ui.separator();

            // Battery Section
            self.render_battery_section(ui);
        });
        
    // Settings window removed
    }
}fn load_icon() -> IconData {
    const ICON_DATA: &[u8] = include_bytes!("../rhelper.ico");
    
    if let Ok(image) = image::load_from_memory(ICON_DATA) {
        let image = image.resize_exact(32, 32, image::imageops::FilterType::Lanczos3).to_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        
        IconData {
            rgba,
            width,
            height,
        }
    } else {
        // Create a simple fallback icon if embedded data can't be loaded
        let size = 32;
        let mut rgba = vec![0u8; (size * size * 4) as usize];
        
        // Create a simple colored square as fallback
        for i in 0..(size * size) as usize {
            let base = i * 4;
            rgba[base] = 0;     // R
            rgba[base + 1] = 150; // G
            rgba[base + 2] = 255; // B
            rgba[base + 3] = 255; // A
        }
        
        IconData {
            rgba,
            width: size,
            height: size,
        }
    }
}

#[cfg(windows)]
fn set_windows_app_id() {
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
    use windows::core::PCWSTR;
    
    let app_id = "RHelper.Application.0.3.2\0".encode_utf16().collect::<Vec<u16>>();
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(PCWSTR(app_id.as_ptr()));
    }
}

#[cfg(not(windows))]
fn set_windows_app_id() {
    // No-op on non-Windows platforms
}

fn main() -> Result<(), eframe::Error> {
    // Set Windows application ID for proper taskbar icon handling
    set_windows_app_id();
    
    // Calculate initial window height (GPU features disabled by default)
    let initial_height = 150.0 + (4.0 * 80.0) + (5.0 * 5.0); // base + 4 sections + separators
    
    // Create the eframe app options
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([450.0, initial_height])
            .with_resizable(false)
            .with_title("R-Helper v0.3.2")
            .with_icon(load_icon())
            .with_always_on_top()
            .with_active(true),
        ..Default::default()
    };

    // Run the eframe app
    eframe::run_native(
    "R-Helper v0.3.2",
        options,
        Box::new(move |cc| {
            // Schedule removal of always-on-top after a short delay
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(500));
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
            });
            
            Box::new(RazerGuiApp::new())
        }),
    )
}
