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
    let source = brand_source(
        ui.ctx(),
        "brand-lockup",
        include_bytes!("../assets/brand-lockup.png"),
    );
    let (area, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 110.0),
        egui::Sense::hover(),
    );
    let aspect = source.width() as f32 / source.height() as f32;
    let width = area.width().min(area.height() * aspect);
    let rect = egui::Rect::from_center_size(area.center(), egui::vec2(width, width / aspect));
    let texture = scaled_texture(ui.ctx(), "brand-lockup", &source, rect.size());
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
    let source = brand_source(
        ui.ctx(),
        "about-presentation",
        include_bytes!("../assets/brand-presentation.png"),
    );
    let width = ui.available_width().min(560.0);
    let height = width * source.height() as f32 / source.width() as f32;
    let texture = scaled_texture(
        ui.ctx(),
        "about-presentation",
        &source,
        egui::vec2(width, height),
    );
    ui.add(
        egui::Image::new(&texture)
            .fit_to_exact_size(egui::vec2(width, height))
            .corner_radius(6),
    );
}

// Decode once; resample only when the physical display size (including DPI) changes.
fn brand_source(
    ctx: &eframe::egui::Context,
    name: &str,
    bytes: &[u8],
) -> std::sync::Arc<image::RgbaImage> {
    let id = eframe::egui::Id::new((name, "source"));
    if let Some(source) = ctx.data(|data| data.get_temp::<std::sync::Arc<image::RgbaImage>>(id)) {
        return source;
    }
    let source = std::sync::Arc::new(
        image::load_from_memory(bytes)
            .expect("Invalid brand image")
            .to_rgba8(),
    );
    ctx.data_mut(|data| data.insert_temp(id, source.clone()));
    source
}

fn scaled_texture(
    ctx: &eframe::egui::Context,
    name: &str,
    source: &image::RgbaImage,
    logical: eframe::egui::Vec2,
) -> eframe::egui::TextureHandle {
    use eframe::egui;
    let size = [
        (logical.x * ctx.pixels_per_point()).ceil().max(1.0) as u32,
        (logical.y * ctx.pixels_per_point()).ceil().max(1.0) as u32,
    ];
    let id = egui::Id::new((name, "scaled"));
    if let Some((previous, texture)) =
        ctx.data(|data| data.get_temp::<([u32; 2], egui::TextureHandle)>(id))
    {
        if previous == size {
            return texture;
        }
    }
    let pixels = resample(source, size);
    let texture = ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(
            [size[0] as usize, size[1] as usize],
            pixels.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|data| data.insert_temp(id, (size, texture.clone())));
    texture
}

fn resample(source: &image::RgbaImage, size: [u32; 2]) -> image::RgbaImage {
    // Filter premultiplied colors to avoid dark fringes around transparent contours.
    let premultiplied = image::Rgba32FImage::from_fn(source.width(), source.height(), |x, y| {
        let pixel = source.get_pixel(x, y);
        let alpha = pixel[3] as f32 / 255.0;
        image::Rgba([
            pixel[0] as f32 / 255.0 * alpha,
            pixel[1] as f32 / 255.0 * alpha,
            pixel[2] as f32 / 255.0 * alpha,
            alpha,
        ])
    });
    let filtered = image::imageops::resize(
        &premultiplied,
        size[0],
        size[1],
        image::imageops::FilterType::Lanczos3,
    );
    image::RgbaImage::from_fn(size[0], size[1], |x, y| {
        let p = filtered.get_pixel(x, y);
        let alpha = p[3].clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return image::Rgba([0; 4]);
        }
        image::Rgba([
            ((p[0] / alpha).clamp(0.0, 1.0) * 255.0).round() as u8,
            ((p[1] / alpha).clamp(0.0, 1.0) * 255.0).round() as u8,
            ((p[2] / alpha).clamp(0.0, 1.0) * 255.0).round() as u8,
            (alpha * 255.0).round() as u8,
        ])
    })
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
    fn downsampling_keeps_transparent_edges_smooth_without_dark_fringes() {
        let source = image::RgbaImage::from_fn(64, 64, |x, y| {
            if x > y {
                image::Rgba([255, 0, 0, 255])
            } else {
                image::Rgba([0; 4])
            }
        });
        let result = resample(&source, [19, 19]);
        let edges: Vec<_> = result.pixels().filter(|p| p[3] > 0 && p[3] < 255).collect();
        assert!(!edges.is_empty());
        for pixel in edges {
            assert_eq!(pixel[0], 255);
            assert_eq!([pixel[1], pixel[2]], [0, 0]);
        }
    }

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
