use eframe::egui;
pub const WIDTH: f32 = 136.0;
pub fn name(priority: i64) -> &'static str {
    match priority {
        0 => "Не скачивать",
        6 => "Высокий",
        7 => "Максимальный",
        255 => "Разный",
        _ => "Обычный",
    }
}
fn color(palette: crate::theme::Palette, priority: i64) -> egui::Color32 {
    match priority {
        1 => palette.green,
        6 => palette.orange,
        7 => palette.red,
        _ => palette.muted,
    }
}
pub fn combo(ui: &mut egui::Ui, id: impl std::hash::Hash, priority: &mut i64) -> egui::Response {
    ui.scope(|ui| {
        // Both callers use the file-manager geometry, regardless of a dialog's compact text style.
        ui.style_mut().override_font_id = Some(egui::FontId::proportional(14.0));
        ui.spacing_mut().button_padding = egui::vec2(4.0, 1.0);
        ui.spacing_mut().interact_size.y = 18.0;
        ui.spacing_mut().icon_width = 14.0;
        ui.spacing_mut().icon_spacing = 4.0;
        let palette = crate::theme::palette(ui.ctx());
        let response = egui::ComboBox::from_id_salt(id)
            .width(WIDTH)
            .selected_text("")
            .show_ui(ui, |ui| {
                ui.with_layout(
                    egui::Layout::top_down(egui::Align::Center).with_cross_justify(true),
                    |ui| {
                        for value in [0, 1, 6, 7] {
                            ui.selectable_value(
                                priority,
                                value,
                                egui::RichText::new(name(value)).color(color(palette, value)),
                            );
                        }
                    },
                );
            })
            .response;
        ui.painter().with_clip_rect(response.rect).text(
            response.rect.center(),
            egui::Align2::CENTER_CENTER,
            name(*priority),
            egui::FontId::proportional(14.0),
            if ui.is_enabled() {
                color(palette, *priority)
            } else {
                palette.muted
            },
        );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, ui.is_enabled(), name(*priority))
        });
        response
    })
    .inner
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn priority_geometry_and_colors_match_in_normal_and_compact_dialog_styles() {
        for theme in crate::theme::Theme::ALL {
            for scale in [1.0, 1.25, 1.5, 2.0] {
                let mut reference = Vec::new();
                for compact in [false, true] {
                    let ctx = egui::Context::default();
                    theme.apply(&ctx);
                    ctx.set_zoom_factor(scale);
                    let mut buttons = Vec::new();
                    let output = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(600.0 * scale, 400.0 * scale),
                            )),
                            ..Default::default()
                        },
                        |ctx| {
                            egui::CentralPanel::default().show(ctx, |ui| {
                                if compact {
                                    ui.style_mut().override_font_id =
                                        Some(egui::FontId::proportional(12.0));
                                    ui.spacing_mut().button_padding = egui::vec2(6.0, 2.0);
                                }
                                for (index, mut priority) in
                                    [0, 1, 6, 7, 255].into_iter().enumerate()
                                {
                                    let response = combo(ui, index, &mut priority);
                                    buttons.push((priority, response.rect));
                                }
                            });
                        },
                    );
                    if !compact {
                        reference = buttons.iter().map(|(_, rect)| rect.size()).collect();
                    }
                    for (index, (priority, rect)) in buttons.iter().enumerate() {
                        assert_eq!(rect.size(), reference[index]);
                        assert_eq!(rect.width(), WIDTH);
                        let text = output
                            .shapes
                            .iter()
                            .find_map(|shape| match &shape.shape {
                                egui::Shape::Text(text)
                                    if text.galley.text() == name(*priority) =>
                                {
                                    Some(text)
                                }
                                _ => None,
                            })
                            .unwrap();
                        let label = egui::Rect::from_min_size(text.pos, text.galley.size());
                        assert!(
                            (label.center().x - rect.center().x).abs() < 0.1
                                && (label.center().y - rect.center().y).abs() < 0.1
                        );
                        assert_eq!(
                            text.galley.job.sections[0].format.color,
                            color(theme.palette(), *priority)
                        );
                        assert!(rect.contains_rect(label));
                    }
                }
            }
        }
    }
    #[test]
    fn dropdown_selects_maximum_from_mixed_state_without_changing_size() {
        let ctx = egui::Context::default();
        let mut priority = 255;
        let mut button = egui::Pos2::ZERO;
        let mut option = egui::Pos2::ZERO;
        let mut size = egui::Vec2::ZERO;
        for frame in 0..6 {
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(600.0, 400.0),
                )),
                ..Default::default()
            };
            if frame > 0 && frame != 3 {
                let pos = if frame <= 2 { button } else { option };
                input.events = vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: frame == 1 || frame == 4,
                        modifiers: egui::Modifiers::NONE,
                    },
                ];
            }
            let output = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let response = combo(ui, "test-priority", &mut priority);
                    if frame == 0 {
                        button = response.rect.center();
                        size = response.rect.size();
                    }
                    assert_eq!(response.rect.size(), size);
                });
            });
            if frame == 3 {
                let labels: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::Shape::Text(text)
                            if [0, 1, 6, 7]
                                .iter()
                                .any(|value| text.galley.text() == name(*value)) =>
                        {
                            Some(text)
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(labels.len(), 4);
                let center = labels[0].pos.x + labels[0].galley.size().x / 2.0;
                for (label, value) in labels.iter().zip([0, 1, 6, 7]) {
                    assert!(
                        (label.pos.x + label.galley.size().x / 2.0 - center).abs() < 0.6,
                        "all priority options must share a horizontal center"
                    );
                    assert_eq!(
                        label.galley.job.sections[0].format.color,
                        color(crate::theme::palette(&ctx), value)
                    );
                }
                option = output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == "Максимальный" => {
                            Some(text.pos + text.galley.size() / 2.0)
                        }
                        _ => None,
                    })
                    .expect("priority menu");
            }
        }
        assert_eq!(priority, 7);
    }
}
