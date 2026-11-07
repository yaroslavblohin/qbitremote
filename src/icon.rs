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
    let rect = pixel_rect(
        ui.ctx(),
        egui::Rect::from_center_size(area.center(), egui::vec2(width, width / aspect)),
    );
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
        "about-forest",
        include_bytes!("../assets/brand-forest.png"),
    );
    let width = ui.available_width().min(560.0);
    let height = width * source.height() as f32 / source.width() as f32;
    let texture = scaled_texture(
        ui.ctx(),
        "about-presentation",
        &source,
        egui::vec2(width, height),
    );
    let response = ui.add(
        egui::Image::new(&texture)
            .fit_to_exact_size(egui::vec2(width, height))
            .corner_radius(6),
    );
    let logo = brand_source(
        ui.ctx(),
        "original-lockup",
        include_bytes!("../assets/brand-lockup.png"),
    );
    let logo_width = width * 0.90;
    let rect = pixel_rect(
        ui.ctx(),
        egui::Rect::from_center_size(
            response.rect.center(),
            egui::vec2(
                logo_width,
                logo_width * logo.height() as f32 / logo.width() as f32,
            ),
        ),
    );
    let texture = scaled_texture(ui.ctx(), "about-lockup", &logo, rect.size());
    ui.painter().image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

// Keep each prepared texel on one physical pixel instead of filtering it a second time.
fn pixel_rect(ctx: &eframe::egui::Context, rect: eframe::egui::Rect) -> eframe::egui::Rect {
    use eframe::egui;
    let scale = ctx.pixels_per_point();
    egui::Rect::from_min_size(
        egui::pos2(
            (rect.left() * scale).round() / scale,
            (rect.top() * scale).round() / scale,
        ),
        egui::vec2(
            (rect.width() * scale).floor().max(1.0) / scale,
            (rect.height() * scale).floor().max(1.0) / scale,
        ),
    )
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
        (logical.x * ctx.pixels_per_point()).round().max(1.0) as u32,
        (logical.y * ctx.pixels_per_point()).round().max(1.0) as u32,
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
    let logo = name == "brand-lockup" || name == "about-lockup";
    let pixels = if logo { sharpen_logo(&pixels) } else { pixels };
    let texture = ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(
            [size[0] as usize, size[1] as usize],
            pixels.as_raw(),
        ),
        if logo {
            egui::TextureOptions::NEAREST
        } else {
            egui::TextureOptions::LINEAR
        },
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

// Mild unsharp mask at final display resolution. Preserve coverage exactly so
// transparent contours stay smooth and do not grow a dark halo.
fn sharpen_logo(source: &image::RgbaImage) -> image::RgbaImage {
    let premultiplied = image::Rgba32FImage::from_fn(source.width(), source.height(), |x, y| {
        let p = source.get_pixel(x, y);
        let a = p[3] as f32 / 255.0;
        image::Rgba([
            p[0] as f32 / 255.0 * a,
            p[1] as f32 / 255.0 * a,
            p[2] as f32 / 255.0 * a,
            a,
        ])
    });
    let blurred = image::imageops::blur(&premultiplied, 0.6);
    image::RgbaImage::from_fn(source.width(), source.height(), |x, y| {
        let original = source.get_pixel(x, y);
        let mut result = *original;
        let blur = blurred.get_pixel(x, y);
        if original[3] > 0 && blur[3] > 0.0 {
            for channel in 0..3 {
                let local = (blur[channel] / blur[3]).clamp(0.0, 1.0) * 255.0;
                let value = original[channel] as f32;
                result[channel] = (value + 0.35 * (value - local)).clamp(0.0, 255.0).round() as u8;
            }
        }
        result
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
    // Preserve the approved artwork verbatim; small application icons remain qb-only.
    let lockup = image::open("assets/brand-lockup-source.png")
        .unwrap()
        .to_rgba8();
    std::fs::copy("assets/brand-lockup-source.png", "assets/brand-lockup.png").unwrap();
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
    fn approved_lockup_is_preserved_with_transparency() {
        let source = include_bytes!("../assets/brand-lockup-source.png");
        assert_eq!(
            source.as_slice(),
            include_bytes!("../assets/brand-lockup.png").as_slice()
        );
        let image = image::load_from_memory(source).unwrap().to_rgba8();
        assert!(image.width() > image.height() * 2);
        assert!(image.pixels().any(|pixel| pixel[3] == 0));
        assert!(image.pixels().any(|pixel| pixel[3] == 255));
    }

    #[test]
    fn downsampling_keeps_transparent_edges_smooth_without_dark_fringes() {
        let source = image::RgbaImage::from_fn(64, 64, |x, y| {
            if x > y {
                image::Rgba([255, 0, 0, 255])
            } else {
                image::Rgba([0; 4])
            }
        });
        let result = sharpen_logo(&resample(&source, [19, 19]));
        let edges: Vec<_> = result.pixels().filter(|p| p[3] > 0 && p[3] < 255).collect();
        assert!(!edges.is_empty());
        for pixel in edges {
            assert_eq!(pixel[0], 255);
            assert_eq!([pixel[1], pixel[2]], [0, 0]);
        }
    }

    #[test]
    fn sharpening_preserves_coverage_and_increases_only_local_edge_contrast() {
        let source = image::RgbaImage::from_fn(32, 16, |x, _| {
            let value = if x < 16 { 90 } else { 150 };
            image::Rgba([value, value, value, 255])
        });
        let sharp = sharpen_logo(&source);
        assert!(sharp.get_pixel(15, 8)[0] < 90);
        assert!(sharp.get_pixel(16, 8)[0] > 150);
        assert_eq!(sharp.get_pixel(4, 8), source.get_pixel(4, 8));
        assert!(source
            .pixels()
            .zip(sharp.pixels())
            .all(|(a, b)| a[3] == b[3]));
    }
    #[test]
    fn prepared_logo_is_cached_by_physical_size_and_uses_one_to_one_sampling() {
        let ctx = egui::Context::default();
        let source = image::RgbaImage::from_pixel(16, 8, image::Rgba([120, 180, 240, 255]));
        let mut previous_id = None;
        for (frame, ppp) in [1.0, 1.0, 1.5, 1.5, 2.0].into_iter().enumerate() {
            ctx.set_pixels_per_point(ppp);
            let output = ctx.run(egui::RawInput::default(), |ctx| {
                let texture =
                    scaled_texture(ctx, "brand-lockup", &source, egui::vec2(200.0, 100.0));
                if let Some(id) = previous_id {
                    if frame == 1 || frame == 3 {
                        assert_eq!(texture.id(), id);
                    } else {
                        assert_ne!(texture.id(), id);
                    }
                }
                previous_id = Some(texture.id());
            });
            let upload = output
                .textures_delta
                .set
                .iter()
                .find(|(id, _)| Some(*id) == previous_id);
            if frame == 1 || frame == 3 {
                assert!(upload.is_none());
            } else {
                let delta = &upload.unwrap().1;
                assert_eq!(
                    delta.image.size(),
                    [(200.0 * ppp) as usize, (100.0 * ppp) as usize]
                );
                assert_eq!(delta.options.magnification, egui::TextureFilter::Nearest);
                assert_eq!(delta.options.minification, egui::TextureFilter::Nearest);
            }
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
                assert!((logo.center().y - area.center().y).abs() <= 1.0 / ctx.pixels_per_point());
                assert!((logo.center().x - area.center().x).abs() <= 1.0 / ctx.pixels_per_point());
                assert!(area.contains_rect(logo));
            }
        }
    }
}
