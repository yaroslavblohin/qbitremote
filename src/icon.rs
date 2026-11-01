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

/// One centered lockup; visible glyph bounds determine the optical alignment.
pub fn header(ui: &mut eframe::egui::Ui) {
    use eframe::egui;
    let size = (63.0 * ui.ctx().pixels_per_point())
        .ceil()
        .clamp(63.0, 252.0) as u32;
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
    let named = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(egui::Id::new("brand-font")))
        .unwrap_or(false);
    let family = if named {
        egui::FontFamily::Name("qbit-wordmark".into())
    } else {
        egui::FontFamily::Proportional
    };
    let color = crate::theme::palette(ui.ctx()).text;
    let mut qbit = ui.painter().layout_no_wrap(
        "qBit".into(),
        egui::FontId::new(20.0, family.clone()),
        color,
    );
    let mut remote =
        ui.painter()
            .layout_no_wrap("Remote".into(), egui::FontId::proportional(20.0), color);
    let initial_ink = qbit.mesh_bounds.union(
        remote
            .mesh_bounds
            .translate(egui::vec2(qbit.size().x + 4.0, 0.0)),
    );
    let scale = ((ui.available_width() - 73.0) / initial_ink.width()).min(1.0);
    if scale < 1.0 {
        qbit = ui.painter().layout_no_wrap(
            "qBit".into(),
            egui::FontId::new(20.0 * scale, family),
            color,
        );
        remote = ui.painter().layout_no_wrap(
            "Remote".into(),
            egui::FontId::proportional(20.0 * scale),
            color,
        );
    }
    let offset = egui::vec2(qbit.size().x + 4.0, 0.0);
    let ink = qbit.mesh_bounds.union(remote.mesh_bounds.translate(offset));
    let width = 63.0 + 10.0 + ink.width();
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 70.0), egui::Sense::hover());
    let left = rect.center().x - width / 2.0;
    let logo = egui::Rect::from_center_size(
        egui::pos2(left + 31.5, rect.center().y),
        egui::vec2(63.0, 63.0),
    );
    ui.painter().image(
        texture.id(),
        logo,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    let text_pos = egui::pos2(left + 73.0 - ink.min.x, rect.center().y - ink.center().y);
    ui.painter().galley(text_pos, qbit, color);
    ui.painter().galley(text_pos + offset, remote, color);
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
    let master = image::open("assets/logo-master.png").expect("Missing logo master PNG");
    let raster = master
        .resize_exact(256, 256, image::imageops::FilterType::Lanczos3)
        .to_rgba8();
    std::fs::write("assets/logo.rgba", raster.as_raw()).unwrap();
    for (name, size) in [("logo", 512_u32), ("logo-small", 32), ("logo-tray", 16)] {
        master
            .resize_exact(size, size, image::imageops::FilterType::Lanczos3)
            .save(format!("assets/{name}.png"))
            .unwrap();
    }
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
                            egui::vec2((width + 16.0) * zoom, 200.0 * zoom),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            area = egui::Rect::from_min_size(
                                ui.available_rect_before_wrap().min,
                                egui::vec2(ui.available_width(), 70.0),
                            );
                            header(ui);
                        });
                    },
                );
                let mut ink = egui::Rect::NOTHING;
                let mut logo = egui::Rect::NOTHING;
                for shape in output.shapes {
                    match shape.shape {
                        egui::Shape::Text(text)
                            if matches!(text.galley.text(), "qBit" | "Remote") =>
                        {
                            ink = ink.union(text.galley.mesh_bounds.translate(text.pos.to_vec2()));
                        }
                        egui::Shape::Mesh(mesh)
                            if mesh.texture_id != egui::TextureId::default() =>
                        {
                            logo = mesh.calc_bounds();
                        }
                        _ => {}
                    }
                }
                assert!(logo.is_positive() && ink.is_positive());
                assert!((logo.center().y - ink.center().y).abs() < 0.1);
                assert!((logo.union(ink).center().x - area.center().x).abs() < 0.1);
                assert!(area.contains_rect(logo.union(ink)));
            }
        }
    }
}
