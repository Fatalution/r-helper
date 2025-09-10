use eframe::egui::{self, Color32, Layout, Align, RichText};
use librazer::types::{PerfMode, CpuBoost, GpuBoost};


// Color constants for better maintainability
const AC_SELECTED_COLOR: Color32 = Color32::from_rgb(0, 120, 60);
const AC_UNSELECTED_COLOR: Color32 = Color32::from_rgb(60, 80, 40);
const BATTERY_SELECTED_COLOR: Color32 = Color32::from_rgb(140, 70, 0);
const BATTERY_UNSELECTED_COLOR: Color32 = Color32::from_rgb(80, 60, 40);
const ORANGE_COLOR: Color32 = Color32::from_rgb(255, 165, 0);
// Muted green for disabled-but-active Custom state
const CUSTOM_ACTIVE_FILL: Color32 = Color32::from_rgb(40, 80, 55);
const CUSTOM_ACTIVE_STROKE: Color32 = Color32::from_rgb(70, 130, 90);

// Actions that can be triggered from the performance UI
#[derive(Debug, Clone, PartialEq)]
pub enum PerformanceAction {
    None,
    SetPerformanceMode(String),
    ToggleHidden,
    SetCpuBoost(CpuBoost),
    SetGpuBoost(GpuBoost),
}

// Renders the performance section UI

pub fn render_performance_section(
    ui: &mut egui::Ui,
    current_performance_mode: &str,
    ac_power: bool,
    available_modes: &[PerfMode],
    base_modes: &[PerfMode],
    show_probe_button: bool,
    current_cpu_boost: CpuBoost,
    current_gpu_boost: GpuBoost,
) -> PerformanceAction {
    let mut action = PerformanceAction::None;
    
    ui.group(|ui| {
        render_performance_header(ui, ac_power, show_probe_button);
        ui.separator();
      
        // Performance Mode Selection
        action = render_performance_modes(ui, current_performance_mode, ac_power, available_modes, base_modes);
            
        // Custom boost controls (visible when in Custom mode OR when debug flag enabled so UI can be tested)
        let show_custom_controls = current_performance_mode == "Custom" || show_probe_button; // reusing debug flag param
        if show_custom_controls {
            ui.add_space(6.0);
            if let Some(custom_action) = render_custom_boosts(ui, ac_power, current_cpu_boost, current_gpu_boost, current_performance_mode == "Custom", show_probe_button) {
                action = custom_action;
            }
        }
    });

    action
}

// Renders CPU / GPU boost selectors when Custom is active (or debug mode to preview UI)
fn render_custom_boosts(
    ui: &mut egui::Ui,
    ac_power: bool,
    current_cpu: CpuBoost,
    current_gpu: GpuBoost,
    custom_active: bool,
    debug_mode: bool,
) -> Option<PerformanceAction> {
    let mut out = None;
    // CPU row: left side label + standard boosts, right-aligned Undervolt (debug only)
    let row_height = ui.spacing().interact_size.y;
    let full_width = ui.available_width();
    ui.allocate_ui_with_layout(egui::Vec2::new(full_width, row_height), Layout::right_to_left(Align::Center), |ui| {
        // Right-most: Undervolt (debug)
        if debug_mode {
            let boost = CpuBoost::Undervolt;
            let label = "Undervolt";
            let selected = boost == current_cpu;
            let color = get_button_color(ac_power, selected);
            let mut btn = egui::Button::new(egui::RichText::new(label).color(Color32::WHITE));
            btn = btn
                .fill(if selected { color } else { Color32::TRANSPARENT })
                .stroke(egui::Stroke::new(1.0, color));
            let response = ui.add_enabled(custom_active, btn);
            if response.clicked() && !selected { out = Some(PerformanceAction::SetCpuBoost(boost)); }
            if !custom_active {
                response.on_hover_text("Firmware preset (Undervolt). Behavior not fully understood; fixed internal setting. Activate Custom mode to apply.");
            } else {
                response.on_hover_text("Firmware preset (Undervolt). Behavior not fully understood; fixed internal setting.");
            }
        }
        // Left group: label + standard boosts
        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.add(egui::Label::new("CPU").selectable(false));
            for boost in [CpuBoost::Low, CpuBoost::Medium, CpuBoost::High, CpuBoost::Boost] {
                let label = format!("{:?}", boost);
                let selected = boost == current_cpu;
                let color = get_button_color(ac_power, selected);
                let mut btn = egui::Button::new(egui::RichText::new(&label).color(Color32::WHITE));
                btn = btn
                    .fill(if selected { color } else { Color32::TRANSPARENT })
                    .stroke(egui::Stroke::new(1.0, color));
                let response = ui.add_enabled(custom_active, btn);
                if response.clicked() && !selected { out = Some(PerformanceAction::SetCpuBoost(boost)); }
                if !custom_active { response.on_hover_text("Activate Custom mode to apply"); }
            }
        });
    });

    // GPU row
    ui.horizontal(|ui| {
        ui.add(egui::Label::new("GPU").selectable(false));
        for boost in [GpuBoost::Low, GpuBoost::Medium, GpuBoost::High] {
            let label = format!("{:?}", boost);
            let selected = boost == current_gpu;
            let color = get_button_color(ac_power, selected);
            let mut btn = egui::Button::new(egui::RichText::new(&label).color(Color32::WHITE));
            btn = btn
                .fill(if selected { color } else { Color32::TRANSPARENT })
                .stroke(egui::Stroke::new(1.0, color));
            let response = ui.add_enabled(custom_active, btn);
            if response.clicked() && !selected { out = Some(PerformanceAction::SetGpuBoost(boost)); }
            if !custom_active { response.on_hover_text("Activate Custom mode to apply"); }
        }
    });

    out
}

// Renders the performance section header with power status
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

// Renders the performance mode selection buttons
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
        if ui.ctx().data(|d| d.get_temp::<bool>("perf_toggle_hidden".into()).unwrap_or(false)) {
            ui.ctx().data_mut(|d| d.remove::<bool>("perf_toggle_hidden".into()));
            action = PerformanceAction::ToggleHidden;
        }
        let base_vec: Vec<PerfMode> = base_modes.iter().cloned().collect();
        let showing_hidden = available_modes.iter().any(|m| !base_vec.contains(m));

    // Left-aligned standard modes (exclude Custom)
    let mut rendered: Vec<PerfMode> = Vec::new();
        for mode in &ordered_modes {
            if available_modes.contains(mode) && *mode != PerfMode::Custom {
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

        // Right-aligned Custom button
        if available_modes.contains(&PerfMode::Custom) {
            let width = ui.available_width();
            let height = ui.spacing().interact_size.y;
            ui.allocate_ui_with_layout(egui::Vec2::new(width, height), Layout::right_to_left(Align::Min), |ui| {
                let custom_str = format!("{:?}", PerfMode::Custom);
                let selected = current_performance_mode == custom_str;
                let fill_color = if selected { CUSTOM_ACTIVE_FILL } else { Color32::TRANSPARENT };
                let stroke_color = if selected { CUSTOM_ACTIVE_STROKE } else { Color32::from_gray(80) };
                let btn = egui::Button::new(RichText::new(&custom_str).color(Color32::WHITE))
                    .fill(fill_color)
                    .stroke(egui::Stroke::new(1.0, stroke_color));
                let response = ui.add(btn);
                if response.clicked() && !selected { action = PerformanceAction::SetPerformanceMode(custom_str); }
                if selected { response.on_hover_text("Custom mode active"); }
                else { response.on_hover_text("Switch to Custom mode"); }
            });
        }
    });
    
    action
}

// Gets the appropriate button color based on power state and selection
fn get_button_color(ac_power: bool, selected: bool) -> Color32 {
    match (ac_power, selected) {
        (true, true) => AC_SELECTED_COLOR,
        (true, false) => AC_UNSELECTED_COLOR,
        (false, true) => BATTERY_SELECTED_COLOR,
        (false, false) => BATTERY_UNSELECTED_COLOR,
    }
}
