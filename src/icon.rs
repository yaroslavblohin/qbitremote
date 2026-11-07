#[path = "icon_pixels.rs"]
mod pixels;
pub use pixels::rgba;
#[path = "brand_vector.rs"]
mod vector;

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
    let (area, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 110.0),
        egui::Sense::hover(),
    );
    let aspect = vector::aspect();
    let width = area.width().min(area.height() * aspect);
    let rect = pixel_rect(
        ui.ctx(),
        egui::Rect::from_center_size(area.center(), egui::vec2(width, width / aspect)),
    );
    let texture = vector_texture(ui.ctx(), "brand-lockup", rect.size());
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
    let logo_width = width * 0.90;
    let rect = pixel_rect(
        ui.ctx(),
        egui::Rect::from_center_size(
            response.rect.center(),
            egui::vec2(logo_width, logo_width / vector::aspect()),
        ),
    );
    let texture = vector_texture(ui.ctx(), "about-vector", rect.size());
    ui.painter().image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

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
fn vector_texture(
    ctx: &eframe::egui::Context,
    name: &str,
    logical: eframe::egui::Vec2,
) -> eframe::egui::TextureHandle {
    use eframe::egui;
    let size = [
        (logical.x * ctx.pixels_per_point()).round().max(1.0) as u32,
        (logical.y * ctx.pixels_per_point()).round().max(1.0) as u32,
    ];
    let id = egui::Id::new((name, "vector-size"));
    if let Some((previous, texture)) =
        ctx.data(|data| data.get_temp::<([u32; 2], egui::TextureHandle)>(id))
    {
        if previous == size {
            return texture;
        }
    }
    let pixels = vector::raster(size[0], size[1]);
    let texture = ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(
            [size[0] as usize, size[1] as usize],
            pixels.as_raw(),
        ),
        egui::TextureOptions::NEAREST,
    );
    ctx.data_mut(|data| data.insert_temp(id, (size, texture.clone())));
    texture
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
    let lockup = vector::raster(1600, (1600.0 / vector::aspect()).round() as u32);
    lockup.save("assets/brand-lockup.png").unwrap();
    let mut forest = image::open("assets/brand-forest.png").unwrap().to_rgba8();
    let width = (forest.width() as f32 * 0.90) as u32;
    let height = width * lockup.height() / lockup.width();
    let overlay = vector::raster(width, height);
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
    fn vector_texture_reuses_same_dpi_and_rerenders_when_pixel_size_changes() {
        let ctx = egui::Context::default();
        let mut previous_id = None;
        for (frame, ppp) in [1.0, 1.0, 1.5, 1.5, 2.0].into_iter().enumerate() {
            ctx.set_pixels_per_point(ppp);
            let output = ctx.run(egui::RawInput::default(), |ctx| {
                let texture = vector_texture(ctx, "cache-test", egui::vec2(200.0, 100.0));
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
                assert_eq!(
                    upload.unwrap().1.image.size(),
                    [(200.0 * ppp) as usize, (100.0 * ppp) as usize]
                );
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
