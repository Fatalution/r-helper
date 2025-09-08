use eframe::egui::{self, Layout, Align, Color32, RichText};

// Constants for RPM ranges and calculations
const MIN_RPM_FOR_COLOR: f32 = 1900.0;
const MAX_RPM_FOR_COLOR: f32 = 5000.0;
const MIN_MANUAL_RPM: u16 = 2000;
// Increased to 5500 to expose full supported range (command layer allows 0..=5500)
const MAX_MANUAL_RPM: u16 = 5500;
const RPM_STEP: f64 = 100.0;
const DARK_GREEN_MAX: u8 = 120;
const ORANGE_MAX: u8 = 100;

/// Actions that can be triggered from the fan UI
#[derive(Debug, Clone, PartialEq)]
pub enum FanAction {
    /// No action requested
    None,
    /// Set fan to automatic mode (from button click)
    SetAutoMode,
    /// Set fan to manual mode (from button click)
    SetManualMode(u16),
    /// Adjust RPM in existing manual mode (from slider)
    SetManualRpm(u16),
    /// Slider is being dragged with current RPM value (for status display)
    SliderDragging(u16),
}

/// Renders the fan control section UI
/// 
/// # Arguments
/// * `ui` - The egui UI context
/// * `fan_speed` - The current fan speed mode string
/// * `fan_actual_rpm` - The current actual fan RPM reading
/// * `fan_set_rpm` - The current set fan RPM (what user configured)
/// * `manual_fan_rpm` - Mutable reference to manual fan RPM setting
/// * `show_status_messages` - Whether to show additional status information
/// 
/// # Returns
/// The action requested by the user, if any
pub fn render_fan_section(
    ui: &mut egui::Ui,
    fan_speed: &str,
    fan_actual_rpm: Option<u16>,
    fan_set_rpm: Option<u16>,
    manual_fan_rpm: &mut u16,
    show_status_messages: bool,
) -> FanAction {
    let mut action = FanAction::None;
    
    ui.group(|ui| {
        render_fan_header(ui, fan_actual_rpm, fan_set_rpm, show_status_messages);
        ui.separator();
        
        // Fan Mode Selection
        action = render_fan_mode_controls(ui, fan_speed, *manual_fan_rpm);
        
        // Manual RPM Slider (shown only in manual mode)
        if fan_speed.eq_ignore_ascii_case("manual") {
            if let Some(manual_action) = render_manual_fan_controls(ui, manual_fan_rpm) {
                action = manual_action;
            }
        }
        
        render_current_status(ui, fan_speed);
    });
    
    action
}

/// Renders the fan section header with live RPM display
fn render_fan_header(ui: &mut egui::Ui, fan_actual_rpm: Option<u16>, fan_set_rpm: Option<u16>, show_status_messages: bool) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new("🌀 Fan Control").selectable(false));
        
        // RPM displays (right-aligned)
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Current (actual) RPM
            if let Some(actual_rpm) = fan_actual_rpm {
                let rpm_color = calculate_rpm_color(actual_rpm);
                ui.add(egui::Label::new(RichText::new(format!("{} RPM", actual_rpm))
                         .color(rpm_color)).selectable(false));
            } else {
                ui.add(egui::Label::new(RichText::new("N/A")).selectable(false));
            }
            
            // Set RPM (only show when status messages are enabled)
            if show_status_messages {
                if let Some(set_rpm) = fan_set_rpm {
                    ui.add(egui::Label::new(RichText::new(format!("Set: {} |", set_rpm))
                             .color(Color32::LIGHT_GRAY)).selectable(false));
                } else {
                    ui.add(egui::Label::new(RichText::new("Set: Auto |")
                             .color(Color32::LIGHT_GRAY)).selectable(false));
                }
            }
        });
    });
}

/// Renders fan mode selection buttons
fn render_fan_mode_controls(ui: &mut egui::Ui, fan_speed: &str, manual_fan_rpm: u16) -> FanAction {
    let mut action = FanAction::None;
    
    ui.horizontal(|ui| {
        let auto_selected = fan_speed.eq_ignore_ascii_case("auto");
        if ui.selectable_label(auto_selected, "Auto").clicked() && !auto_selected {
            action = FanAction::SetAutoMode;
        }
        
        let manual_selected = fan_speed.eq_ignore_ascii_case("manual");
        if ui.selectable_label(manual_selected, "Manual").clicked() && !manual_selected {
            action = FanAction::SetManualMode(manual_fan_rpm);
        }
    });
    
    action
}

/// Renders manual fan RPM controls
fn render_manual_fan_controls(ui: &mut egui::Ui, manual_fan_rpm: &mut u16) -> Option<FanAction> {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new("RPM:").selectable(false));
        let fan_response = ui.add(
            egui::Slider::new(manual_fan_rpm, MIN_MANUAL_RPM..=MAX_MANUAL_RPM)
                .step_by(RPM_STEP)
        );
        
        if fan_response.dragged() || fan_response.has_focus() {
            // Slider is being actively used - show live RPM status
            Some(FanAction::SliderDragging(*manual_fan_rpm))
        } else if fan_response.drag_stopped() || fan_response.lost_focus() {
            // User finished adjusting - this is just an RPM change, not a mode change
            Some(FanAction::SetManualRpm(*manual_fan_rpm))
        } else {
            None
        }
    }).inner
}

/// Renders the current fan status
fn render_current_status(ui: &mut egui::Ui, fan_speed: &str) {
    ui.add(egui::Label::new(format!("Current: {}", fan_speed)).selectable(false));
}

/// Calculates the appropriate color for RPM display based on fan speed
fn calculate_rpm_color(actual_rpm: u16) -> Color32 {
    // Calculate color based on RPM (1900 = dark green, 5000 = orange-red)
    let normalized_rpm = ((actual_rpm as f32 - MIN_RPM_FOR_COLOR) / (MAX_RPM_FOR_COLOR - MIN_RPM_FOR_COLOR)).clamp(0.0, 1.0);
    
    // Dark green to orange-red gradient
    let green_component = ((1.0 - normalized_rpm) * DARK_GREEN_MAX as f32) as u8;
    let red_component = (normalized_rpm * 255.0) as u8;
    let orange_component = (normalized_rpm * 165.0) as u8; // Add orange component
    
    Color32::from_rgb(red_component, green_component, orange_component.min(ORANGE_MAX))
}
