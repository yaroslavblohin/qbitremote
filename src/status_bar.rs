use eframe::egui::{self, Color32, FontId, Rect};

pub struct State<'a> {
    pub connected: bool,
    pub connecting: bool,
    pub server: &'a str,
    pub version: &'a str,
    pub download: u64,
    pub upload: u64,
    pub free: Option<u64>,
    pub selected: usize,
}

pub fn show(ctx: &egui::Context, state: &State<'_>) {
    let p = crate::theme::palette(ctx);
    egui::TopBottomPanel::bottom("status")
        .frame(
            egui::Frame::new()
                .fill(p.sidebar)
                .inner_margin(egui::Margin::symmetric(14, 0)),
        )
        .show(ctx, |ui| {
            paint(ui, state);
        });
}

fn paint(ui: &mut egui::Ui, state: &State<'_>) -> [Rect; 5] {
    let p = crate::theme::palette(ui.ctx());
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 32.0), egui::Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);
    painter.line_segment(
        [rect.left_top(), rect.right_top()],
        egui::Stroke::new(1.0_f32, p.border),
    );
    let font = FontId::proportional(12.0);
    let gap = if width < 600.0 { 14.0 } else { 22.0 };
    let free = if state.connected { state.free } else { None };
    let speed = |n| {
        if state.connected {
            format!("{}/с", crate::model::bytes(n))
        } else {
            "—".into()
        }
    };
    let values = [
        speed(state.download),
        speed(state.upload),
        free.map(crate::model::bytes).unwrap_or_else(|| "—".into()),
    ];
    let caption = if width >= 600.0 {
        "Свободно"
    } else {
        ""
    };
    let mut right = rect.right();
    let mut cells = [Rect::NOTHING; 3];
    for index in (0..3).rev() {
        let text = painter.layout_no_wrap(values[index].clone(), font.clone(), p.text);
        let caption_galley = painter.layout_no_wrap(
            if index == 2 { caption } else { "" }.into(),
            font.clone(),
            p.muted,
        );
        let caption_width = if caption_galley.size().x > 0.0 {
            caption_galley.size().x + 7.0
        } else {
            0.0
        };
        let size = egui::vec2(20.0 + caption_width + text.size().x, 32.0);
        let cell = Rect::from_min_size(egui::pos2(right - size.x, rect.top()), size);
        cells[index] = cell;
        let center = cell.left_center() + egui::vec2(6.0, 0.0);
        if index == 2 {
            let disk = Rect::from_center_size(center, egui::vec2(13.0, 10.0));
            painter.rect_stroke(
                disk,
                2.0,
                egui::Stroke::new(1.0_f32, p.muted),
                egui::StrokeKind::Inside,
            );
            painter.line_segment(
                [
                    disk.left_bottom() + egui::vec2(2.0, -3.0),
                    disk.right_bottom() + egui::vec2(-2.0, -3.0),
                ],
                egui::Stroke::new(1.0_f32, p.muted),
            );
        } else {
            crate::app::draw_icon(
                &painter,
                center,
                if index == 0 { 1 } else { 2 },
                if state.connected {
                    if index == 0 {
                        p.blue
                    } else {
                        p.green
                    }
                } else {
                    p.muted
                },
            );
        }
        let y = cell.center().y - text.size().y / 2.0;
        painter.galley(egui::pos2(cell.left() + 20.0, y), caption_galley, p.muted);
        painter.galley(
            egui::pos2(cell.left() + 20.0 + caption_width, y),
            text,
            if index == 2 {
                crate::server_space::color(free, &p)
            } else {
                p.text
            },
        );
        ui.interact(cell, ui.id().with(("status-metric", index)), egui::Sense::hover()).on_hover_text(match index {
            0 => "Общая скорость скачивания всех торрентов сервера",
            1 => "Общая скорость отдачи всех торрентов сервера",
            _ if !state.connected => "Подключите сервер для получения свободного места",
            _ if free.is_none() => "Сервер пока не сообщил свободное место или запрос не удался",
            _ => "Свободное место на диске папки загрузки по умолчанию удалённого сервера. Обновляется каждые 3 секунды.\nМеньше 10 ГиБ — оранжевый, меньше 1 ГиБ — красный. Другие папки могут находиться на других дисках.",
        });
        right = cell.left() - gap;
    }
    let separator = cells[2].left() - gap / 2.0;
    painter.line_segment(
        [
            egui::pos2(separator, rect.center().y - 6.0),
            egui::pos2(separator, rect.center().y + 6.0),
        ],
        egui::Stroke::new(1.0_f32, p.border),
    );
    let selected_text = painter.layout_no_wrap(
        format!(
            "{}: {}",
            if width < 600.0 {
                "Выбр."
            } else {
                "Выбрано"
            },
            state.selected
        ),
        font.clone(),
        p.muted,
    );
    let selected_width = selected_text.size().x.min((right - rect.left()).max(0.0));
    let selected_rect = Rect::from_min_max(
        egui::pos2((right - selected_width).max(rect.left()), rect.top()),
        egui::pos2(right.max(rect.left()), rect.bottom()),
    );
    painter.with_clip_rect(selected_rect).galley(
        selected_rect.left_center() - egui::vec2(0.0, selected_text.size().y / 2.0),
        selected_text,
        p.muted,
    );
    ui.interact(
        selected_rect,
        ui.id().with("selected-count"),
        egui::Sense::hover(),
    )
    .on_hover_text(format!("Выбрано торрентов: {}", state.selected));
    right = (selected_rect.left() - gap).max(rect.left());
    let connection =
        Rect::from_min_max(rect.min, egui::pos2(right.max(rect.left()), rect.bottom()));
    let status = if state.connected {
        "Подключено"
    } else if state.connecting {
        "Подключение…"
    } else {
        "Нет подключения"
    };
    let text = if width >= 820.0 && !state.server.is_empty() {
        format!(
            "{status}  ·  {}  ·  Windows {}",
            state.server,
            env!("CARGO_PKG_VERSION")
        )
    } else {
        format!("{status}  ·  Windows {}", env!("CARGO_PKG_VERSION"))
    };
    let clipped = painter.with_clip_rect(connection);
    clipped.circle_filled(
        connection.left_center() + egui::vec2(3.0, 0.0),
        3.0,
        if state.connected {
            p.green
        } else if state.connecting {
            p.orange
        } else {
            p.muted
        },
    );
    let mut job =
        egui::text::LayoutJob::simple(text, font, p.text, (connection.width() - 16.0).max(0.0));
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let text = clipped.layout_job(job);
    clipped.galley(
        connection.left_center() + egui::vec2(14.0, -text.size().y / 2.0),
        text,
        Color32::WHITE,
    );
    ui.interact(
        connection,
        ui.id().with("connection-status"),
        egui::Sense::hover(),
    )
    .on_hover_text(format!(
        "{status}\n{}{}",
        state.server,
        if state.connected {
            format!("\nqBittorrent {}", state.version)
        } else {
            String::new()
        }
    ));
    [connection, selected_rect, cells[0], cells[1], cells[2]]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metrics_stay_on_one_baseline_without_overlap_at_different_widths_and_scales() {
        for theme in crate::theme::Theme::ALL {
            for zoom in [1.0, 1.5, 2.0] {
                for width in [430.0, 600.0, 860.0, 1500.0, 2300.0] {
                    let ctx = egui::Context::default();
                    theme.apply(&ctx);
                    ctx.set_zoom_factor(zoom);
                    let state = State {
                        connected: true,
                        connecting: false,
                        server: "Очень длинное название удалённого сервера",
                        version: "5.3.0",
                        download: 9_000_000_000,
                        upload: 9_000_000_000,
                        free: Some(9_000_000_000_000),
                        selected: 9999,
                    };
                    let _ = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(width * zoom, 200.0 * zoom),
                            )),
                            ..Default::default()
                        },
                        |ctx| {
                            egui::CentralPanel::default().show(ctx, |ui| {
                                let bounds = ui.available_rect_before_wrap();
                                let cells = paint(ui, &state);
                                assert!(cells[2].width() > 60.0);
                                for pair in cells.windows(2) {
                                    assert!(pair[0].right() <= pair[1].left());
                                }
                                for cell in cells {
                                    assert!(bounds.contains_rect(cell));
                                    assert_eq!(cell.height(), 32.0);
                                }
                            });
                        },
                    );
                }
            }
        }
    }
}
