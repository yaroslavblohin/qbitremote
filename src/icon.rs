#[path = "icon_pixels.rs"]
mod pixels;
pub use pixels::rgba;

pub fn window() -> eframe::egui::IconData {
    eframe::egui::IconData {
        rgba: rgba(64),
        width: 64,
        height: 64,
    }
}
