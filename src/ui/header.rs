use eframe::egui::{self, Layout, Align, Color32, RichText};
use crate::system::SystemSpecs;
use crate::messaging::{MessageManager, MessageType};
use librazer::device::Device;

const FADE_START_TIME: f32 = 3.0;
const FADE_DURATION: f32 = 2.0;
const FULL_ALPHA: u8 = 255;

/// Renders the application header with device name and status messages
pub fn render_header(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    loading: bool,
    system_specs: &SystemSpecs,
    device: &Option<Device>,
    message_manager: &MessageManager,
    detecting_device: bool,
) {
    ui.horizontal(|ui| {
        // Device name
        render_device_name(ui, device, system_specs);
        
        // Status messages and connection status
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if loading {
                ui.spinner();
            }
            
            // Status/warning messages
            render_status_messages(ui, ctx, message_manager, device, detecting_device);
        });
    });
}

/// Renders device name section
fn render_device_name(ui: &mut egui::Ui, device: &Option<Device>, system_specs: &SystemSpecs) {
    let device_text = if device.is_some() || system_specs.device_model != "Unknown" {
        if system_specs.device_model != "Unknown" {
            format!("💻 {}", system_specs.device_model)
        } else {
            "💻 Connected Device".to_string()
        }
    } else {
        "💻 No Razer Device".to_string()
    };
    
    ui.add(egui::Label::new(egui::RichText::new(device_text).heading()).selectable(false));
}

/// Renders status messages with fade animation
fn render_status_messages(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    message_manager: &MessageManager,
    device: &Option<Device>,
    detecting_device: bool,
) {
    if let Some(current_message) = message_manager.get_current_message() {
        let elapsed = current_message.age_seconds();
        
        // Calculate fade and apply to message
        let (base_color, icon) = get_message_style_from_type(&current_message.message_type);
        let alpha = calculate_fade_alpha(elapsed);
        let faded_color = apply_alpha_to_color(base_color, alpha);
        
        ui.add(egui::Label::new(RichText::new(format!("{} {}", icon, current_message.content)).color(faded_color)).selectable(false));
        
        // Request repaint for smooth animation
        if current_message.should_fade() {
            ctx.request_repaint();
        }
    } else {
        // Show connection status when no device detected
        if device.is_none() {
            if detecting_device {
                ui.add(egui::Label::new(RichText::new("🔎 Detecting device…").color(Color32::LIGHT_BLUE)).selectable(false));
                ctx.request_repaint();
            } else {
                ui.add(egui::Label::new(RichText::new("❌ No device detected").color(Color32::RED)).selectable(false));
            }
        }
    }
}

/// Message style based on type
fn get_message_style_from_type(message_type: &MessageType) -> (Color32, &'static str) {
    match message_type {
        MessageType::Info => (Color32::LIGHT_BLUE, "ℹ"),
        MessageType::Error => (Color32::RED, "⚠"),
    }
}

/// Calculates alpha value for fade animation
fn calculate_fade_alpha(elapsed: f32) -> f32 {
    if elapsed < FADE_START_TIME {
        1.0
    } else {
        let fade_progress = (elapsed - FADE_START_TIME) / FADE_DURATION;
        (1.0 - fade_progress).max(0.0)
    }
}

/// Applies alpha transparency to color
fn apply_alpha_to_color(base_color: Color32, alpha: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        base_color.r(),
        base_color.g(), 
        base_color.b(),
        (FULL_ALPHA as f32 * alpha) as u8,
    )
}
