use eframe::egui::{self, Layout, Align};
use std::process::Command;

/// Renders the application footer with version info and controls
pub fn render_footer(
    ui: &mut egui::Ui,
    status_messages: &mut bool,
) {
    // Add vertical padding for better spacing
    ui.add_space(8.0);
    
    ui.horizontal(|ui| {
        render_version_info(ui);
        ui.separator();
        render_status_toggle(ui, status_messages);
        
        // GitHub button on the right side
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ui.button("🌐 GitHub").clicked() {
                let _ = Command::new("cmd")
                    .args(&["/c", "start", "https://github.com"])
                    .spawn();
            }
        });
    });
    
    // Add bottom padding for balance
    ui.add_space(8.0);
}

/// Renders the application version information
fn render_version_info(ui: &mut egui::Ui) {
    let label = egui::Label::new("Made with ♥ by Fatalution").selectable(false).sense(egui::Sense::click());
    if ui.add(label).clicked() {
        let _ = Command::new("cmd")
            .args(&["/c", "start", "https://paypal.me/fatalutionDE"])
            .spawn();
    }
}

/// Renders the status messages toggle
fn render_status_toggle(ui: &mut egui::Ui, status_messages: &mut bool) {
    ui.checkbox(status_messages, "🐛 Status messages");
}
