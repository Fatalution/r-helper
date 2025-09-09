use eframe::egui::{self, Color32, Layout, Align, RichText};
use librazer::types::PerfMode;

// Constants for power limits per performance mode
const BATTERY_POWER_LIMITS: (u8, u16) = (25, 40);
const SILENT_POWER_LIMITS: (u8, u16) = (35, 110);
const BALANCED_POWER_LIMITS: (u8, u16) = (45, 130);
const PERFORMANCE_POWER_LIMITS: (u8, u16) = (65, 160);
const HYPERBOOST_POWER_LIMITS: (u8, u16) = (65, 175);

// Color constants for better maintainability
const AC_SELECTED_COLOR: Color32 = Color32::from_rgb(0, 120, 60);
const AC_UNSELECTED_COLOR: Color32 = Color32::from_rgb(60, 80, 40);
const BATTERY_SELECTED_COLOR: Color32 = Color32::from_rgb(140, 70, 0);
const BATTERY_UNSELECTED_COLOR: Color32 = Color32::from_rgb(80, 60, 40);
const ORANGE_COLOR: Color32 = Color32::from_rgb(255, 165, 0);
// Muted green for disabled-but-active Custom state
const CUSTOM_ACTIVE_FILL: Color32 = Color32::from_rgb(40, 80, 55);
const CUSTOM_ACTIVE_STROKE: Color32 = Color32::from_rgb(70, 130, 90);

/// Actions that can be triggered from the performance UI
#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceAction {
    None,
    SetPerformanceMode(String),
    ToggleHidden,
}

/// Renders the performance section UI
/// 
/// # Arguments
/// * `ui` - The egui UI context
/// * `current_performance_mode` - The currently active performance mode
/// * `ac_power` - Whether the device is running on AC power
/// * `device_model` - The device model name for conditional feature display
/// * `gpu_models` - The list of GPU models for hardware-specific features
/// * `available_modes` - The list of available performance modes for this device
/// 
/// # Returns
/// The action requested by the user, if any
pub fn render_performance_section(
    ui: &mut egui::Ui,
    current_performance_mode: &str,
    ac_power: bool,
    device_model: &str,
    gpu_models: &[String],
    available_modes: &[PerfMode],
    base_modes: &[PerfMode],
    show_probe_button: bool,
) -> PerformanceAction {
    let mut action = PerformanceAction::None;
    
    ui.group(|ui| {
        render_performance_header(ui, ac_power, show_probe_button);
        ui.separator();
        
        // Performance Mode Selection
    action = render_performance_modes(ui, current_performance_mode, ac_power, available_modes, base_modes);
        
        // Power Limits and Profile Management (only for supported devices)
        render_power_limits_and_controls(ui, current_performance_mode, device_model, gpu_models, &mut action);
    });
    
    action
}

/// Renders the performance section header with power status
fn render_performance_header(ui: &mut egui::Ui, ac_power: bool, show_probe_button: bool) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new("🚀 Performance Mode").selectable(false));
        
        // Power status indicator
        let (power_icon, power_color) = if ac_power {
            ("🔌", Color32::GREEN)
        } else {
            ("🔋", ORANGE_COLOR)
        };
        
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if show_probe_button {
                if ui.small_button("👁").on_hover_text("Show/Hide hidden modes").clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp("perf_toggle_hidden".into(), true));
                }
            }
            ui.add(egui::Label::new(RichText::new(power_icon).color(power_color)).selectable(false));
            ui.add(egui::Label::new(RichText::new(if ac_power { "AC Power" } else { "Battery" })).selectable(false));
        });
    });
}

/// Renders the performance mode selection buttons
fn render_performance_modes(
    ui: &mut egui::Ui, 
    current_performance_mode: &str, 
    ac_power: bool,
    available_modes: &[PerfMode],
    base_modes: &[PerfMode],
) -> PerformanceAction {
    let mut action = PerformanceAction::None;
    
    ui.horizontal(|ui| {
        // Define the desired order for performance modes
        let ordered_modes = [
            PerfMode::Battery,
            PerfMode::Silent,
            PerfMode::Balanced,
            PerfMode::Performance,
            PerfMode::Hyperboost,
        ];
        
        // Render main performance modes (active ones) in preferred order
        let mut rendered: Vec<PerfMode> = Vec::new();
        if ui.ctx().data(|d| d.get_temp::<bool>("perf_toggle_hidden".into()).unwrap_or(false)) {
            ui.ctx().data_mut(|d| d.remove::<bool>("perf_toggle_hidden".into()));
            action = PerformanceAction::ToggleHidden;
        }
    let base_vec: Vec<PerfMode> = base_modes.iter().cloned().collect();
    let showing_hidden = available_modes.iter().any(|m| !base_vec.contains(m));
        for mode in &ordered_modes {
            if available_modes.contains(mode) {
                let mode_str = format!("{:?}", mode);
                let selected = current_performance_mode == mode_str;
                let button_color = get_button_color(ac_power, selected);
                let is_hidden = showing_hidden && !base_vec.contains(mode);
                let mut btn = egui::Button::new(RichText::new(&mode_str).color(if is_hidden && !selected { Color32::from_gray(160) } else { Color32::WHITE }));
                btn = btn.fill(if selected { button_color } else { Color32::TRANSPARENT })
                    .stroke(egui::Stroke::new(1.0, if is_hidden && !selected { Color32::from_gray(90) } else { button_color }));
                let response = ui.add(btn);
                if response.clicked() && !selected { action = PerformanceAction::SetPerformanceMode(mode_str); }
                if is_hidden { response.on_hover_text("Hidden / unsupported by descriptor"); }
                rendered.push(*mode);
            }
        }
        
        // Render any additional modes not in our preferred order (e.g., future modes like Turbo)
        for mode in available_modes {
            if *mode != PerfMode::Custom && !rendered.contains(mode) {
                let mode_str = format!("{:?}", mode);
                let selected = current_performance_mode == mode_str;
                let button_color = get_button_color(ac_power, selected);
                let is_hidden = showing_hidden && !base_vec.contains(mode);
                let mut btn = egui::Button::new(RichText::new(&mode_str).color(if is_hidden && !selected { Color32::from_gray(160) } else { Color32::WHITE }));
                btn = btn.fill(if selected { button_color } else { Color32::TRANSPARENT })
                    .stroke(egui::Stroke::new(1.0, if is_hidden && !selected { Color32::from_gray(90) } else { button_color }));
                let response = ui.add(btn);
                if response.clicked() && !selected { action = PerformanceAction::SetPerformanceMode(mode_str); }
                if is_hidden { response.on_hover_text("Hidden / unsupported by descriptor"); }
            }
        }
        
        // Add spacing before Custom mode
        ui.add_space(20.0);
        
        // Render Custom mode on the right side (inactive)
        if available_modes.contains(&PerfMode::Custom) {
            let custom_str = format!("{:?}", PerfMode::Custom);
            let is_active_custom = current_performance_mode == custom_str;

            let fill_color = if is_active_custom { CUSTOM_ACTIVE_FILL } else { Color32::from_gray(40) };
            let stroke_color = if is_active_custom { CUSTOM_ACTIVE_STROKE } else { Color32::from_gray(80) };

            let response = ui.add_enabled(
                false, // keep non-interactive
                egui::Button::new(&custom_str)
                    .fill(fill_color)
                    .stroke(egui::Stroke::new(1.0, stroke_color))
            );

            if is_active_custom {
                response.on_hover_text("Custom is active (set externally). Control disabled here");
            } else {
                response.on_hover_text("Custom mode not yet implemented");
            }
        }
    });
    
    action
}

/// Gets the appropriate button color based on power state and selection
fn get_button_color(ac_power: bool, selected: bool) -> Color32 {
    match (ac_power, selected) {
        (true, true) => AC_SELECTED_COLOR,
        (true, false) => AC_UNSELECTED_COLOR,
        (false, true) => BATTERY_SELECTED_COLOR,
        (false, false) => BATTERY_UNSELECTED_COLOR,
    }
}

/// Renders power limits display and profile management controls
fn render_power_limits_and_controls(
    ui: &mut egui::Ui, 
    current_performance_mode: &str, 
    device_model: &str,
    gpu_models: &[String],
    _action: &mut PerformanceAction
) {
    ui.horizontal(|ui| {
        // Show CPU and GPU power limits only for supported devices
        if should_show_power_limits(device_model, gpu_models) {
            render_power_limits_display(ui, current_performance_mode);
        } else {
            // For unsupported devices, show current mode only
            ui.add(egui::Label::new(format!("Mode: {}", current_performance_mode)).selectable(false));
        }
        
        // Profile management removed - using automatic AC/battery switching instead
    });
}

/// Displays the power limits for the current performance mode
fn render_power_limits_display(ui: &mut egui::Ui, current_performance_mode: &str) {
    let (cpu_power, gpu_power) = get_power_limits(current_performance_mode);
    
    let display_text = if cpu_power > 0 && gpu_power > 0 {
        format!("CPU: {}W  |  GPU: {}W", cpu_power, gpu_power)
    } else {
        format!("Current: {}", current_performance_mode)
    };
    
    ui.add(egui::Label::new(display_text).selectable(false));
}

/// Gets the power limits for a given performance mode
fn get_power_limits(mode: &str) -> (u8, u16) {
    match mode {
        "Battery" => BATTERY_POWER_LIMITS,
        "Silent" => SILENT_POWER_LIMITS,
        "Balanced" => BALANCED_POWER_LIMITS,
        "Performance" => PERFORMANCE_POWER_LIMITS,
        "Hyperboost" => HYPERBOOST_POWER_LIMITS,
        _ => (0, 0), // Unknown mode
    }
}

/// Determines if power limits should be displayed for this device configuration
/// Only shows power limits for Razer Blade 16 2025 with RTX 5080 or 5090
fn should_show_power_limits(device_model: &str, gpu_models: &[String]) -> bool {
    // Check if this is a Razer Blade 16 2025
    let is_blade_16_2025 = device_model.contains("Razer Blade 16") && device_model.contains("(2025)");
    
    if !is_blade_16_2025 {
        return false;
    }
    
    // Check if it has RTX 5080 or 5090
    let has_supported_gpu = gpu_models.iter().any(|gpu| {
        gpu.contains("RTX 5080") || gpu.contains("RTX 5090")
    });
    
    has_supported_gpu
}
