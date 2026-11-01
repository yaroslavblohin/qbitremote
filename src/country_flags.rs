use eframe::egui;

#[path = "country_flags_data.rs"]
mod data;
const PIRATE: &[u8] = include_bytes!("../assets/flags/pirate.png");

fn asset(code: &str) -> (&'static str, &'static [u8]) {
    let code = code.trim().to_ascii_lowercase();
    data::FLAGS
        .binary_search_by_key(&code.as_str(), |(code, _)| *code)
        .map(|index| data::FLAGS[index])
        .unwrap_or(("pirate", PIRATE))
}

pub fn country(ui: &mut egui::Ui, code: &str, name: &str) {
    let (key, bytes) = asset(code);
    let id = egui::Id::new(("country-flag", key));
    let texture = ui
        .ctx()
        .data(|data| data.get_temp::<egui::TextureHandle>(id));
    let texture = texture.unwrap_or_else(|| {
        let image = image::load_from_memory(bytes)
            .expect("validated embedded flag")
            .to_rgba8();
        let size = [image.width() as usize, image.height() as usize];
        let texture = ui.ctx().load_texture(
            format!("flag-{key}"),
            egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
            egui::TextureOptions::LINEAR,
        );
        ui.ctx()
            .data_mut(|data| data.insert_temp(id, texture.clone()));
        texture
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        let (slot, response) = ui.allocate_exact_size(egui::vec2(22.0, 16.0), egui::Sense::hover());
        let source = texture.size_vec2();
        let scale = (slot.width() / source.x).min(slot.height() / source.y);
        let rect = egui::Rect::from_center_size(slot.center(), source * scale);
        ui.painter().image(
            texture.id(),
            rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        response.on_hover_text(if key == "pirate" {
            "Страна неизвестна · Весёлый Роджер".into()
        } else {
            format!("{} ({})", name, key.to_ascii_uppercase())
        });
        ui.add(
            egui::Label::new(if name.is_empty() {
                "Неизвестно"
            } else {
                name
            })
            .truncate(),
        )
        .on_hover_text(name);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn country_codes_are_case_insensitive_and_unknown_codes_use_pirate() {
        assert_eq!(asset(" RU ").0, "ru");
        assert_eq!(asset("cn").0, "cn");
        assert_eq!(asset("xk").0, "xk");
        for code in ["", "??", "zz", "Russia", "../ru", "💀"] {
            assert_eq!(asset(code).0, "pirate");
            assert_eq!(asset(code).1, PIRATE);
        }
    }
    #[test]
    fn all_embedded_flags_decode_and_have_enough_pixels_for_hidpi() {
        assert!(data::FLAGS.len() >= 250);
        for pair in data::FLAGS.windows(2) {
            assert!(pair[0].0 < pair[1].0);
        }
        for (code, bytes) in data::FLAGS
            .iter()
            .copied()
            .chain(std::iter::once(("pirate", PIRATE)))
        {
            let image = image::load_from_memory(bytes).unwrap_or_else(|e| panic!("{code}: {e}"));
            assert!(image.width() >= 64 && image.height() > 0, "{code}");
        }
    }
}
