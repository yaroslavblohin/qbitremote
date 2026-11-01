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

/// Same raster source and alpha as the tray and executable resources.
pub fn sidebar(ui: &mut eframe::egui::Ui) {
    use eframe::egui;
    let size = (42.0 * ui.ctx().pixels_per_point())
        .ceil()
        .clamp(42.0, 168.0) as u32;
    let id = egui::Id::new(("brand-logo", size));
    let texture = ui
        .ctx()
        .data(|data| data.get_temp::<egui::TextureHandle>(id))
        .unwrap_or_else(|| {
            let texture = ui.ctx().load_texture(
                format!("qbit-remote-brand-{size}"),
                egui::ColorImage::from_rgba_unmultiplied([size as usize; 2], &rgba(size)),
                egui::TextureOptions::LINEAR,
            );
            ui.ctx()
                .data_mut(|data| data.insert_temp(id, texture.clone()));
            texture
        });
    ui.add(egui::Image::new(&texture).fit_to_exact_size(egui::vec2(42.0, 42.0)));
}
#[cfg(feature = "ui-smoke")]
pub fn export_branding() {
    std::fs::create_dir_all("assets").unwrap();
    for (name, size) in [("logo", 512_u32), ("logo-small", 32), ("logo-tray", 16)] {
        image::save_buffer(
            format!("assets/{name}.png"),
            &rgba(size),
            size,
            size,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}
