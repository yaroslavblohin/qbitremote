#[path = "icon_pixels.rs"]
mod pixels;
use eframe::egui;
pub use pixels::rgba;

pub fn window() -> egui::IconData {
    egui::IconData {
        rgba: rgba(64),
        width: 64,
        height: 64,
    }
}
fn monogram(ctx: &egui::Context, physical_width: u32) -> egui::TextureHandle {
    let width = physical_width.max(1);
    let id = egui::Id::new(("qbr-monogram", width));
    if let Some(texture) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return texture;
    }
    let square = image::RgbaImage::from_raw(width, width, rgba(width)).unwrap();
    let top = width / 4;
    let height = (width / 2).max(1);
    let cropped = image::imageops::crop_imm(&square, 0, top, width, height).to_image();
    let texture = ctx.load_texture(
        "qbr",
        egui::ColorImage::from_rgba_unmultiplied(
            [width as usize, height as usize],
            cropped.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| d.insert_temp(id, texture.clone()));
    texture
}
fn font(ctx: &egui::Context, size: f32) -> egui::FontId {
    let family = if ctx
        .data(|d| d.get_temp::<bool>(egui::Id::new("brand-font")))
        .unwrap_or(false)
    {
        egui::FontFamily::Name("qbit-wordmark".into())
    } else {
        egui::FontFamily::Proportional
    };
    egui::FontId::new(size, family)
}
fn lockup(ui: &mut egui::Ui, height: f32, large: bool) {
    let (area, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let width = area.width().min(if large { 300.0 } else { 160.0 });
    let text_size = if large { 32.0 } else { 23.0 };
    let total = width / 2.0 + 8.0 + text_size * 1.2;
    let top = area.center().y - total / 2.0;
    let scale = ui.ctx().pixels_per_point();
    let physical = (width * scale).round() as u32;
    let texture = monogram(ui.ctx(), physical);
    let rect = egui::Rect::from_min_size(
        egui::pos2((area.center().x - width / 2.0).round(), top.round()),
        egui::vec2(width, width / 2.0),
    );
    ui.painter().image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    let color = if ui.visuals().dark_mode {
        egui::Color32::from_rgb(240, 244, 249)
    } else {
        egui::Color32::from_rgb(24, 32, 43)
    };
    ui.painter().text(
        egui::pos2(area.center().x, rect.bottom() + 8.0),
        egui::Align2::CENTER_TOP,
        "qBit Remote",
        font(ui.ctx(), text_size),
        color,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), "qBit Remote")
    });
}
pub fn header(ui: &mut egui::Ui) {
    lockup(ui, 110.0, false);
}
pub fn presentation(ui: &mut egui::Ui) {
    lockup(ui, 220.0, true);
}

#[cfg(feature = "ui-smoke")]
pub fn export_branding() {
    import_branding();
}

#[cfg(feature = "ui-smoke")]
pub fn import_branding() {
    use ab_glyph::{Font, FontRef, PxScale, ScaleFont};
    for (name, size) in [
        ("logo-master", 1024),
        ("logo", 512),
        ("logo-small", 32),
        ("logo-tray", 16),
    ] {
        image::save_buffer(
            format!("assets/{name}.png"),
            &rgba(size),
            size,
            size,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
    std::fs::write("assets/logo.rgba", rgba(256)).unwrap();
    let bytes = std::fs::read("C:/Windows/Fonts/seguisb.ttf")
        .expect("Segoe UI Semibold is required for branding export");
    let face = FontRef::try_from_slice(&bytes).unwrap();
    let scaled = face.as_scaled(PxScale::from(115.0));
    for dark in [false, true] {
        let mut canvas = image::RgbaImage::new(1500, 400);
        let square = image::RgbaImage::from_raw(560, 560, rgba(560)).unwrap();
        let mark = image::imageops::crop_imm(&square, 0, 140, 560, 280).to_image();
        image::imageops::overlay(&mut canvas, &mark, 20, 60);
        let mut x = 620.0;
        let mut previous = None;
        for c in "qBit Remote".chars() {
            let mut glyph = scaled.scaled_glyph(c);
            if let Some(prev) = previous {
                x += scaled.kern(prev, glyph.id);
            }
            glyph.position = ab_glyph::point(x, 235.0);
            x += scaled.h_advance(glyph.id);
            previous = Some(glyph.id);
            if let Some(outline) = face.outline_glyph(glyph) {
                let bounds = outline.px_bounds();
                outline.draw(|gx, gy, coverage| {
                    let px = gx as i32 + bounds.min.x as i32;
                    let py = gy as i32 + bounds.min.y as i32;
                    if px >= 0 && py >= 0 && px < 1500 && py < 400 {
                        let color = if dark { [240, 244, 249] } else { [24, 32, 43] };
                        canvas.put_pixel(
                            px as u32,
                            py as u32,
                            image::Rgba([
                                color[0],
                                color[1],
                                color[2],
                                (coverage * 255.0).round() as u8,
                            ]),
                        );
                    }
                });
            }
        }
        let name = if dark {
            "assets/brand-lockup-dark.png"
        } else {
            "assets/brand-lockup.png"
        };
        canvas.save(name).unwrap();
        if !dark {
            canvas.save("assets/brand-lockup-source.png").unwrap();
            let mut background =
                image::RgbaImage::from_pixel(1500, 400, image::Rgba([248, 249, 251, 255]));
            image::imageops::overlay(&mut background, &canvas, 0, 0);
            background.save("assets/brand-presentation.png").unwrap();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn monogram_texture_is_cached_and_has_no_square_background() {
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let a = monogram(ctx, 160);
            let b = monogram(ctx, 160);
            assert_eq!(a.id(), b.id());
            assert_eq!(a.size(), [160, 80]);
            assert_ne!(a.id(), monogram(ctx, 240).id());
        });
        for size in [16, 32, 64] {
            assert_eq!(rgba(size)[3], 0);
        }
    }
}
