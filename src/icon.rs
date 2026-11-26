#[path = "icon_pixels.rs"]
mod pixels;
use eframe::egui;
pub use pixels::system_rgba;

fn rgba(size: u32) -> Vec<u8> {
    pixels::rasterize(size, 320.0)
}

pub fn window() -> egui::IconData {
    // An empty icon tells eframe to use the macOS bundle's Dock icon.
    #[cfg(target_os = "macos")]
    if crate::macos_settings::has_app_bundle() {
        return egui::IconData::default();
    }

    egui::IconData {
        rgba: system_rgba(64),
        width: 64,
        height: 64,
    }
}

#[cfg(target_os = "macos")]
pub fn menu_bar_rgba() -> (Vec<u8>, u32, u32) {
    const WIDTH: u32 = 64;
    const HEIGHT: u32 = 40;
    let mut pixels = rgba(WIDTH);
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel[..3].fill(0);
    }
    let row_bytes = (WIDTH * 4) as usize;
    let top = ((WIDTH - HEIGHT) / 2) as usize;
    (
        pixels[top * row_bytes..(top + HEIGHT as usize) * row_bytes].to_vec(),
        WIDTH,
        HEIGHT,
    )
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
fn lockup(ui: &mut egui::Ui, min_height: f32, large: bool) {
    let available_width = ui.available_width();
    let width = if large {
        available_width.min(300.0)
    } else {
        (available_width - 4.0).max(1.0)
    };
    let text_size = if large { 32.0 } else { 26.0 };
    // Compensate for the texture's bottom padding in proportion to the monogram width.
    let gap = -width * (3.0 / 160.0);
    let total = width / 2.0 + gap + text_size * 1.2;
    let height = min_height.max(total + 4.0);
    let (area, response) =
        ui.allocate_exact_size(egui::vec2(available_width, height), egui::Sense::hover());
    let top = area.center().y - total / 2.0;
    let scale = ui.ctx().pixels_per_point();
    let physical = (width * scale).round() as u32;
    let texture = monogram(ui.ctx(), physical);
    let rect = egui::Rect::from_min_size(
        egui::pos2((area.center().x - width / 2.0).round(), top.round()),
        egui::vec2(width, width / 2.0),
    );
    // Keep the approved silhouettes crisp; depth is painted behind the original.
    let text_pos = egui::pos2(area.center().x, rect.bottom() + gap);
    let depth = if large { 3.0 } else { 2.0 };
    let shadow = |offset: egui::Vec2, alpha: u8| {
        let alpha = if ui.visuals().dark_mode {
            alpha
        } else {
            alpha / 3
        };
        let color = egui::Color32::from_black_alpha(alpha);
        ui.painter().image(
            texture.id(),
            rect.translate(offset),
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            color,
        );
        ui.painter().text(
            text_pos + offset,
            egui::Align2::CENTER_TOP,
            "qBit Remote",
            font(ui.ctx(), text_size),
            color,
        );
    };
    // A small distributed shadow gives a soft falloff around the short extrusion.
    for y in -2_i32..=2 {
        for x in -2_i32..=2 {
            let weight = (3 - x.abs()) * (3 - y.abs());
            shadow(
                egui::vec2(depth + x as f32, depth + 1.0 + y as f32),
                (weight * 2) as u8,
            );
        }
    }
    for step in (1..=4).rev() {
        shadow(egui::Vec2::splat(depth * step as f32 / 4.0), 80);
    }
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
        text_pos,
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
    lockup(ui, 0.0, false);
}
pub fn presentation(ui: &mut egui::Ui) {
    lockup(ui, 220.0, true);
}

#[cfg(all(windows, feature = "ui-smoke"))]
pub fn export_branding() {
    import_branding();
}

#[cfg(all(windows, feature = "ui-smoke"))]
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
            &system_rgba(size),
            size,
            size,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
    std::fs::write("assets/logo.rgba", system_rgba(256)).unwrap();
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

    #[cfg(target_os = "macos")]
    #[test]
    fn menu_bar_template_keeps_only_the_monogram_opacity() {
        let (icon, width, height) = menu_bar_rgba();
        let pixels = icon.as_chunks::<4>().0;
        assert!(width > height);
        assert_eq!(pixels.len(), (width * height) as usize);
        assert!(pixels.iter().all(|pixel| pixel[..3] == [0, 0, 0]));
        assert!(pixels.iter().any(|pixel| pixel[3] == 255));
        assert!(pixels.iter().any(|pixel| (1..255).contains(&pixel[3])));
        assert!(pixels[..width as usize].iter().all(|pixel| pixel[3] == 0));
        assert!(pixels[pixels.len() - width as usize..]
            .iter()
            .all(|pixel| pixel[3] == 0));
    }
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
