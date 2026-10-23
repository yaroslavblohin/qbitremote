#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;
mod app;
mod columns;
#[cfg(windows)]
mod credentials;
mod icon;
mod model;
mod remote;
#[cfg(feature = "ui-smoke")]
mod smoke;
#[cfg(windows)]
mod tray;

fn main() -> eframe::Result {
    eframe::run_native(
        "qBit Remote",
        eframe::NativeOptions {
            #[cfg(feature = "ui-smoke")]
            persistence_path: Some(std::path::PathBuf::from("dist/ui-smoke-state.ron")),
            viewport: eframe::egui::ViewportBuilder::default()
                .with_icon(icon::window())
                .with_inner_size([1200.0, 780.0])
                .with_min_inner_size([860.0, 580.0]),
            ..Default::default()
        },
        Box::new(|cc| {
            let app = app::ClientApp::new(cc);
            #[cfg(feature = "ui-smoke")]
            let app = smoke::SmokeApp::new(app);
            Ok(Box::new(app))
        }),
    )
}
