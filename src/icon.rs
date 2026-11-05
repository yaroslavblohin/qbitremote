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

/// Full demoscene lockup, centered as one image so icon and wordmark stay aligned.
pub fn header(ui: &mut eframe::egui::Ui) {
    use eframe::egui;
    let id = egui::Id::new("brand-lockup");
    let texture = ui
        .ctx()
        .data(|data| data.get_temp::<egui::TextureHandle>(id))
        .unwrap_or_else(|| {
            let image = image::load_from_memory(include_bytes!("../assets/brand-lockup.png"))
                .expect("Invalid brand lockup")
                .thumbnail(1024, 1024)
                .to_rgba8();
            let texture = ui.ctx().load_texture(
                "brand-lockup",
                egui::ColorImage::from_rgba_unmultiplied(
                    [image.width() as usize, image.height() as usize],
                    image.as_raw(),
                ),
                egui::TextureOptions::LINEAR,
            );
            ui.ctx()
                .data_mut(|data| data.insert_temp(id, texture.clone()));
            texture
        });
    let (area, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 110.0),
        egui::Sense::hover(),
    );
    let aspect = texture.size()[0] as f32 / texture.size()[1] as f32;
    let width = area.width().min(area.height() * aspect);
    let rect = egui::Rect::from_center_size(area.center(), egui::vec2(width, width / aspect));
    ui.painter().image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Label, ui.is_enabled(), "qBit Remote")
    });
}

/// Embedded, cached presentation: does not depend on files beside the EXE.
pub fn presentation(ui: &mut eframe::egui::Ui) {
    use eframe::egui;
    let id = egui::Id::new("about-presentation");
    let texture = ui
        .ctx()
        .data(|data| data.get_temp::<egui::TextureHandle>(id))
        .unwrap_or_else(|| {
            let image = image::load_from_memory(include_bytes!("../assets/brand-presentation.png"))
                .expect("Invalid embedded presentation")
                .to_rgba8();
            let size = [image.width() as usize, image.height() as usize];
            let texture = ui.ctx().load_texture(
                "about-presentation",
                egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
                egui::TextureOptions::LINEAR,
            );
            ui.ctx()
                .data_mut(|data| data.insert_temp(id, texture.clone()));
            texture
        });
    let width = ui.available_width().min(560.0);
    let height = width * texture.size()[1] as f32 / texture.size()[0] as f32;
    ui.add(
        egui::Image::new(&texture)
            .fit_to_exact_size(egui::vec2(width, height))
            .corner_radius(6),
    );
}

#[cfg(feature = "ui-smoke")]
pub fn export_branding() {
    std::fs::create_dir_all("assets").unwrap();
    let master = image::open("assets/logo-master.png").unwrap();
    master
        .resize_exact(512, 512, image::imageops::FilterType::Lanczos3)
        .save("assets/logo.png")
        .unwrap();
    for (name, size) in [("logo-small", 32), ("logo-tray", 16)] {
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

#[cfg(feature = "ui-smoke")]
pub fn import_branding() {
    let master = image::RgbaImage::from_raw(1024, 1024, rgba(1024)).unwrap();
    master.save("assets/logo-master.png").unwrap();
    std::fs::write("assets/logo.rgba", rgba(256)).unwrap();
    for (name, size) in [("logo", 512), ("logo-small", 32), ("logo-tray", 16)] {
        image::save_buffer(
            format!("assets/{name}.png"),
            &rgba(size),
            size,
            size,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
    // Dedicated suffix: trim transparent padding only, never crop letter geometry.
    let source = image::open("assets/brand-suffix.png").unwrap().to_rgba8();
    let mut bounds = (source.width(), source.height(), 0, 0);
    for (x, y, pixel) in source.enumerate_pixels() {
        if pixel[3] > 0 {
            bounds.0 = bounds.0.min(x);
            bounds.1 = bounds.1.min(y);
            bounds.2 = bounds.2.max(x);
            bounds.3 = bounds.3.max(y);
        }
    }
    let text = image::imageops::crop_imm(
        &source,
        bounds.0,
        bounds.1,
        bounds.2 - bounds.0 + 1,
        bounds.3 - bounds.1 + 1,
    )
    .to_image();
    let text = image::imageops::resize(&text, 700, 230, image::imageops::FilterType::Lanczos3);
    let icon = image::RgbaImage::from_raw(900, 900, rgba(900)).unwrap();
    // Trim transparent padding so the suffix sits directly beside the monogram.
    let mut bounds = (900, 900, 0, 0);
    for (x, y, pixel) in icon.enumerate_pixels() {
        if pixel[3] > 0 {
            bounds.0 = bounds.0.min(x);
            bounds.1 = bounds.1.min(y);
            bounds.2 = bounds.2.max(x);
            bounds.3 = bounds.3.max(y);
        }
    }
    let icon = image::imageops::crop_imm(
        &icon,
        bounds.0,
        bounds.1,
        bounds.2 - bounds.0 + 1,
        bounds.3 - bounds.1 + 1,
    )
    .to_image();
    let y = (icon.height() - text.height()) / 2;
    // Match actual contours row by row, rather than their transparent rectangles.
    let mut text_x = 0_i64;
    for row in 0..text.height() {
        let right = (0..icon.width()).rfind(|&x| icon.get_pixel(x, y + row)[3] > 16);
        let left = (0..text.width()).find(|&x| text.get_pixel(x, row)[3] > 16);
        if let (Some(right), Some(left)) = (right, left) {
            text_x = text_x.max(right as i64 + 1 + 12 - left as i64);
        }
    }
    let mut lockup = image::RgbaImage::new(
        (text_x as u32 + text.width()).max(icon.width()),
        icon.height(),
    );
    image::imageops::overlay(&mut lockup, &icon, 0, 0);
    image::imageops::overlay(&mut lockup, &text, text_x, y as i64);
    lockup.save("assets/brand-lockup.png").unwrap();
    let mut forest = image::open("assets/brand-forest.png").unwrap().to_rgba8();
    let width = (forest.width() as f32 * 0.90) as u32;
    let height = width * lockup.height() / lockup.width();
    let overlay = image::imageops::resize(
        &lockup,
        width,
        height,
        image::imageops::FilterType::Lanczos3,
    );
    let x = (forest.width() - width) / 2;
    let y = (forest.height() - height) / 2;
    image::imageops::overlay(&mut forest, &overlay, x as i64, y as i64);
    forest.save("assets/brand-presentation.png").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;
    #[test]
    fn visible_header_artwork_is_centered_at_different_scales_and_widths() {
        for zoom in [1.0, 1.5, 2.0] {
            for width in [190.0, 218.0, 300.0] {
                let ctx = egui::Context::default();
                ctx.set_zoom_factor(zoom);
                let mut area = egui::Rect::NOTHING;
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2((width + 16.0) * zoom, 300.0 * zoom),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            area = egui::Rect::from_min_size(
                                ui.available_rect_before_wrap().min,
                                egui::vec2(ui.available_width(), 110.0),
                            );
                            header(ui);
                        });
                    },
                );

                let mut logo = egui::Rect::NOTHING;
                for shape in output.shapes {
                    match shape.shape {
                        egui::Shape::Mesh(mesh)
                            if mesh.texture_id != egui::TextureId::default() =>
                        {
                            logo = mesh.calc_bounds();
                        }
                        _ => {}
                    }
                }
                assert!(logo.is_positive());
                assert!((logo.center().y - area.center().y).abs() < 0.1);
                assert!((logo.center().x - area.center().x).abs() < 0.1);
                assert!(area.contains_rect(logo));
            }
        }
    }
}
