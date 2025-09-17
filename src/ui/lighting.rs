use eframe::egui;

// Discrete brightness levels that the keyboard actually supports
// Based on testing with Fn+F10/F11 brightness keys
const BRIGHTNESS_LEVELS: &[u8] = &[
    0,   // Step 0
    13,  // Step 1
    28,  // Step 2
    43,  // Step 3
    59,  // Step 4
    74,  // Step 5
    89,  // Step 6
    105, // Step 7
    120, // Step 8
    133, // Step 9
    148, // Step 10
    163, // Step 11
    179, // Step 12
    194, // Step 13
    209, // Step 14
    225, // Step 15
];

/// Actions that can be triggered from the lighting UI
#[derive(Debug, Clone, PartialEq)]
pub struct LightingAction {
    /// Logo lighting mode to set
    pub logo_mode: Option<String>,
    /// Brightness value to set (0-255 raw value)
    pub brightness: Option<u8>,
    /// Whether the lights always on setting was toggled
    pub lights_always_on: bool,
    /// Whether the brightness slider is currently being interacted with
    pub slider_active: Option<bool>,
}

impl Default for LightingAction {
    fn default() -> Self {
        Self { logo_mode: None, brightness: None, lights_always_on: false, slider_active: None }
    }
}

/// Renders the lighting section UI
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `logo_mode` - The current logo lighting mode
/// * `temp_brightness_step` - Mutable reference to brightness step index (0-15)
/// * `lights_always_on` - Mutable reference to lights always on setting
///
/// # Returns
/// The action requested by the user, if any
pub fn render_lighting_section(
    ui: &mut egui::Ui,
    logo_mode: &str,
    temp_brightness_step: &mut usize,
    lights_always_on: &mut bool,
) -> LightingAction {
    let mut action = LightingAction::default();

    ui.group(|ui| {
        ui.add(egui::Label::new("💡 Lighting").selectable(false));
        ui.separator();

        // Logo Mode Selection
        render_logo_mode_selection(ui, logo_mode, &mut action);

        // Brightness Slider
        render_brightness_controls(ui, temp_brightness_step, &mut action);

        // Lights Always On Toggle
        render_always_on_toggle(ui, lights_always_on, &mut action);
    });

    action
}

/// Renders the logo mode selection controls
fn render_logo_mode_selection(ui: &mut egui::Ui, logo_mode: &str, action: &mut LightingAction) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new("Logo Mode:").selectable(false));
        const LOGO_MODES: &[&str] = &["Static", "Breathing", "Off"];

        for mode in LOGO_MODES {
            let selected = logo_mode == *mode;
            if ui.selectable_label(selected, *mode).clicked() && !selected {
                action.logo_mode = Some(mode.to_string());
            }
        }
    });
}

/// Renders the brightness control slider
fn render_brightness_controls(
    ui: &mut egui::Ui,
    temp_brightness_step: &mut usize,
    action: &mut LightingAction,
) {
    ui.horizontal(|ui| {
        ui.add(egui::Label::new("Keyboard Brightness:").selectable(false));

        // Ensure step index is within bounds
        *temp_brightness_step = (*temp_brightness_step).min(BRIGHTNESS_LEVELS.len() - 1);

        let mut step_index = *temp_brightness_step;
        let brightness_response = ui.add(
            egui::Slider::new(&mut step_index, 0..=(BRIGHTNESS_LEVELS.len() - 1))
                .custom_formatter(|val, _| format!("{}", val as usize))
                .custom_parser(|s| s.parse::<f64>().ok()),
        );

        // Check if the value actually changed
        let value_changed = step_index != *temp_brightness_step;
        *temp_brightness_step = step_index;

        // Track slider interaction state
        if brightness_response.dragged() || brightness_response.has_focus() {
            action.slider_active = Some(true);
            // Send brightness command immediately when value changes during interaction
            if value_changed {
                action.brightness = Some(BRIGHTNESS_LEVELS[*temp_brightness_step]);
            }
        } else if brightness_response.drag_stopped() || brightness_response.lost_focus() {
            action.slider_active = Some(false);
            // Send the final brightness value when interaction ends
            if value_changed {
                action.brightness = Some(BRIGHTNESS_LEVELS[*temp_brightness_step]);
            }
        } else if value_changed {
            // Handle cases where value changed without drag (e.g., clicking on slider track)
            action.brightness = Some(BRIGHTNESS_LEVELS[*temp_brightness_step]);
        }
    });
}

/// Renders the always on toggle control
fn render_always_on_toggle(
    ui: &mut egui::Ui,
    lights_always_on: &mut bool,
    action: &mut LightingAction,
) {
    ui.horizontal(|ui| {
        if ui.checkbox(lights_always_on, "Keyboard Backlight Always On").clicked() {
            action.lights_always_on = true;
        }
    });
}

/// Converts raw brightness (0-255) to the closest supported step index
pub fn raw_brightness_to_step_index(brightness: u8) -> usize {
    BRIGHTNESS_LEVELS
        .iter()
        .enumerate()
        .min_by_key(|(_, &level)| (level as i16 - brightness as i16).abs())
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}
