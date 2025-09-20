#![windows_subsystem = "windows"]

mod app;
mod device;
mod messaging;
mod power;
mod system;
mod ui;
mod utils;

use app::RazerGuiApp;
use eframe::egui;
use egui::IconData;

// Dynamic app metadata from Cargo
const APP_NAME: &str = "R-Helper";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

// app::RazerGuiApp implements eframe::App internally
fn load_icon() -> IconData {
    const ICON_DATA: &[u8] = include_bytes!("../rhelper.ico");

    if let Ok(image) = image::load_from_memory(ICON_DATA) {
        let image = image.resize_exact(32, 32, image::imageops::FilterType::Lanczos3).to_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();

        IconData { rgba, width, height }
    } else {
        let size = 32;
        let mut rgba = vec![0u8; (size * size * 4) as usize];

        for i in 0..(size * size) as usize {
            let base = i * 4;
            rgba[base] = 0;
            rgba[base + 1] = 150;
            rgba[base + 2] = 255;
            rgba[base + 3] = 255;
        }

        IconData { rgba, width: size, height: size }
    }
}

#[cfg(windows)]
fn set_windows_app_id() {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;
    // Build a per-version AppUserModelID so taskbar grouping updates with releases
    let app_id =
        format!("RHelper.Application.{}\0", APP_VERSION).encode_utf16().collect::<Vec<u16>>();
    unsafe {
        let _ = SetCurrentProcessExplicitAppUserModelID(PCWSTR(app_id.as_ptr()));
    }
}

#[cfg(not(windows))]
fn set_windows_app_id() {}

fn main() -> Result<(), eframe::Error> {
    set_windows_app_id();
    let initial_height = 500.0;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([450.0, initial_height])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_fullscreen(false)
            .with_title(APP_NAME)
            .with_icon(load_icon())
            .with_always_on_top()
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| {
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(500));
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                    egui::WindowLevel::Normal,
                ));
            });

            let mut app = RazerGuiApp::new();
            app.base_window_height = initial_height as f32;
            Ok(Box::new(app))
        }),
    )
}
