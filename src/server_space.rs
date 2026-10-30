use eframe::egui;
use serde_json::Value;

#[derive(Default)]
pub struct Snapshot {
    pub rid: u64,
    pub free: Option<u64>,
}
impl Snapshot {
    pub fn apply(&mut self, value: &Value) -> Result<Option<u64>, String> {
        let rid = value["rid"]
            .as_u64()
            .ok_or("Некорректный ответ состояния сервера")?;
        if value["full_update"].as_bool() == Some(true) {
            self.free = None;
        }
        if let Some(space) = value
            .get("server_state")
            .and_then(|s| s.get("free_space_on_disk"))
        {
            self.free = space.as_u64();
        }
        self.rid = rid;
        Ok(self.free)
    }
}
#[derive(Debug, PartialEq, Eq)]
enum Level {
    Unknown,
    Normal,
    Low,
    Critical,
}
fn level(free: Option<u64>) -> Level {
    match free {
        None => Level::Unknown,
        Some(n) if n < 1024 * 1024 * 1024 => Level::Critical,
        Some(n) if n < 10 * 1024 * 1024 * 1024 => Level::Low,
        _ => Level::Normal,
    }
}
pub fn badge(
    ui: &mut egui::Ui,
    free: Option<u64>,
    connected: bool,
    compact: bool,
) -> egui::Response {
    let p = crate::theme::palette(ui.ctx());
    let free = if connected { free } else { None };
    let color = match level(free) {
        Level::Unknown => p.muted,
        Level::Normal => p.green,
        Level::Low => p.orange,
        Level::Critical => p.red,
    };
    let caption = if compact {
        "Свободно"
    } else {
        "Свободно на сервере"
    };
    let value = free.map(crate::model::bytes).unwrap_or_else(|| "—".into());
    let font = egui::FontId::proportional(12.0);
    let label = ui
        .painter()
        .layout_no_wrap(caption.into(), font.clone(), p.muted);
    let number = ui.painter().layout_no_wrap(value, font, color);
    let width = (label.size().x + number.size().x + 43.0).min(ui.available_width());
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 22.0), egui::Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, 5.0, color.gamma_multiply(0.10));
    let disk = egui::Rect::from_center_size(
        rect.left_center() + egui::vec2(13.0, 0.0),
        egui::vec2(13.0, 11.0),
    );
    painter.rect_stroke(
        disk,
        2.0,
        egui::Stroke::new(1.0_f32, color),
        egui::StrokeKind::Inside,
    );
    painter.line_segment(
        [
            disk.left_bottom() + egui::vec2(2.0, -4.0),
            disk.right_bottom() + egui::vec2(-2.0, -4.0),
        ],
        egui::Stroke::new(1.0_f32, color),
    );
    painter.circle_filled(disk.right_bottom() + egui::vec2(-3.0, -2.0), 0.8, color);
    let pos = rect.left_center() + egui::vec2(26.0, -label.size().y / 2.0);
    painter.galley(pos, label.clone(), p.muted);
    painter.galley(pos + egui::vec2(label.size().x + 7.0, 0.0), number, color);
    response.on_hover_text(if !connected{"Подключите сервер для получения свободного места."}else if free.is_none(){"Свободное место пока недоступно. Сервер не вернул значение или запрос не удался."}else{"Свободное место на диске папки загрузки по умолчанию удалённого qBittorrent. Обновляется каждые 3 секунды.\nОранжевый: меньше 10 ГиБ; красный: меньше 1 ГиБ.\nДругие папки загрузки могут находиться на других дисках."})
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn retains_unchanged_deltas_and_resets_full_snapshot() {
        let mut s = Snapshot::default();
        assert_eq!(
            s.apply(
                &json!({"rid":1,"full_update":true,"server_state":{"free_space_on_disk":1234}})
            )
            .unwrap(),
            Some(1234)
        );
        assert_eq!(
            s.apply(&json!({"rid":2,"server_state":{"dl_info_speed":10}}))
                .unwrap(),
            Some(1234)
        );
        assert_eq!(s.rid, 2);
        assert_eq!(s.apply(&json!({"rid":3,"full_update":true})).unwrap(), None);
    }
    #[test]
    fn unknown_is_distinct_from_zero_and_large_disks_are_supported() {
        let mut s = Snapshot::default();
        for (value, expected) in [
            (json!(-1), None),
            (json!(0), Some(0)),
            (json!(9_000_000_000_000_u64), Some(9_000_000_000_000)),
        ] {
            assert_eq!(
                s.apply(&json!({"rid":1,"server_state":{"free_space_on_disk":value}}))
                    .unwrap(),
                expected
            );
        }
        assert!(s.apply(&json!({"error":"failed"})).is_err());
        assert_eq!(level(Some(0)), Level::Critical);
        assert_eq!(level(None), Level::Unknown);
        assert_eq!(level(Some(1024 * 1024 * 1024)), Level::Low);
        assert_eq!(level(Some(10 * 1024 * 1024 * 1024)), Level::Normal);
    }
}
