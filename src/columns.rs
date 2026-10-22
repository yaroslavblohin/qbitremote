use crate::model::{bytes, Torrent};
use serde_json::{json, Value};
use std::cmp::Ordering;

#[derive(Clone, Copy)]
pub enum Kind {
    Text,
    Bytes,
    Speed,
    Number,
    Date,
    Duration,
    Ratio,
    Bool,
}
pub struct TorrentColumn {
    pub key: &'static str,
    pub title: &'static str,
    pub kind: Kind,
}
use Kind::*;
macro_rules! columns {
    ($(($key:literal, $title:literal, $kind:ident)),* $(,)?) => {
        pub const COLUMNS: &[TorrentColumn] = &[$(TorrentColumn { key: $key, title: $title, kind: $kind }),*];
    };
}
// Matches the desktop qBittorrent transfer list, using Web API field names.
columns![
    ("name", "Имя", Text),
    ("size", "Размер", Bytes),
    ("progress", "Прогресс", Number),
    ("state", "Состояние", Text),
    ("dlspeed", "↓ Загрузка", Speed),
    ("upspeed", "↑ Отдача", Speed),
    ("priority", "№ очереди", Number),
    ("total_size", "Полный размер", Bytes),
    ("num_seeds", "Сиды", Number),
    ("num_leechs", "Пиры", Number),
    ("eta", "Осталось времени", Duration),
    ("ratio", "Рейтинг", Ratio),
    ("popularity", "Популярность", Ratio),
    ("category", "Категория", Text),
    ("tags", "Теги", Text),
    ("added_on", "Добавлен", Date),
    ("completion_on", "Завершён", Date),
    ("tracker", "Трекер", Text),
    ("dl_limit", "Лимит загрузки", Speed),
    ("up_limit", "Лимит отдачи", Speed),
    ("downloaded", "Скачано", Bytes),
    ("uploaded", "Отдано", Bytes),
    ("downloaded_session", "Скачано за сеанс", Bytes),
    ("uploaded_session", "Отдано за сеанс", Bytes),
    ("amount_left", "Осталось скачать", Bytes),
    ("time_active", "Время активности", Duration),
    ("save_path", "Путь сохранения", Text),
    ("completed", "Готово", Bytes),
    ("ratio_limit", "Лимит рейтинга", Ratio),
    ("seen_complete", "Последний полный источник", Date),
    ("last_activity", "Последняя активность", Date),
    ("availability", "Доступность", Ratio),
    ("download_path", "Временная папка", Text),
    ("infohash_v1", "Инфохеш v1", Text),
    ("infohash_v2", "Инфохеш v2", Text),
    ("reannounce", "Следующий анонс", Duration),
    ("private", "Приватный", Bool),
    ("creation_date", "Создан", Date),
];

pub fn defaults() -> Vec<String> {
    COLUMNS[..6].iter().map(|c| c.key.to_owned()).collect()
}

pub fn move_column(keys: &mut Vec<String>, source: &str, target: &str, after: bool) {
    if source == target || !keys.iter().any(|key| key == target) {
        return;
    }
    if let Some(from) = keys.iter().position(|key| key == source) {
        let key = keys.remove(from);
        let to = keys.iter().position(|key| key == target).unwrap();
        keys.insert(to + usize::from(after), key);
    }
}
pub fn normalize(keys: &mut Vec<String>) {
    keys.retain(|key| COLUMNS.iter().any(|c| c.key == key));
    if keys.is_empty() {
        *keys = defaults();
    }
}
impl TorrentColumn {
    pub fn saved_width(&self, settings: &crate::model::Settings) -> f32 {
        settings
            .column_widths
            .get(self.key)
            .copied()
            .filter(|width| width.is_finite() && *width >= 60.0)
            .unwrap_or_else(|| self.width())
    }
    pub fn width(&self) -> f32 {
        if self.key == "name" {
            310.0
        } else if matches!(self.kind, Date) {
            170.0
        } else {
            (self.title.chars().count() as f32 * 7.5 + 24.0).max(110.0)
        }
    }
    fn value(&self, t: &Torrent) -> Value {
        match self.key {
            "name" => json!(t.name),
            "size" => json!(t.size),
            "progress" => json!(t.progress),
            "state" => json!(t.status()),
            "dlspeed" => json!(t.dlspeed),
            "upspeed" => json!(t.upspeed),
            "num_seeds" => json!(t.num_seeds),
            "num_leechs" => json!(t.num_leechs),
            "ratio" => json!(t.ratio),
            "eta" => json!(t.eta),
            "category" => json!(t.category),
            "save_path" => json!(t.save_path),
            _ => t.extra.get(self.key).cloned().unwrap_or(Value::Null),
        }
    }
    pub fn compare(&self, a: &Torrent, b: &Torrent) -> Ordering {
        let (a, b) = (self.value(a), self.value(b));
        match (a.as_f64(), b.as_f64()) {
            (Some(a), Some(b)) => a.total_cmp(&b),
            _ => match (a.is_null(), b.is_null()) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => a
                    .to_string()
                    .to_lowercase()
                    .cmp(&b.to_string().to_lowercase()),
            },
        }
    }
    pub fn display(&self, t: &Torrent) -> String {
        let v = self.value(t);
        if v.is_null() {
            return "—".into();
        }
        let n = v.as_f64().unwrap_or(0.0);
        if matches!(self.key, "num_seeds" | "num_leechs") {
            let total = t
                .extra
                .get(if self.key == "num_seeds" {
                    "num_complete"
                } else {
                    "num_incomplete"
                })
                .and_then(Value::as_i64)
                .filter(|n| *n >= 0)
                .map(|n| n.to_string())
                .unwrap_or_else(|| "—".into());
            return format!("{n:.0} ({total})");
        }
        if self.key == "priority" && n <= 0.0 {
            return "—".into();
        }
        if self.key == "ratio_limit" && n < 0.0 {
            return if n == -2.0 {
                "Глобальный"
            } else {
                "∞"
            }
            .into();
        }
        if self.key.ends_with("_limit") && matches!(self.kind, Speed) && n <= 0.0 {
            return "∞".into();
        }
        match self.kind {
            Text => v.as_str().filter(|s| !s.is_empty()).unwrap_or("—").into(),
            Bytes => v.as_u64().map(bytes).unwrap_or_else(|| "—".into()),
            Speed => format!("{}/с", bytes(n.max(0.0) as u64)),
            Number => format!("{n:.0}"),
            Ratio => {
                if n < 0.0 {
                    "—".into()
                } else {
                    format!("{n:.2}")
                }
            }
            Bool => v
                .as_bool()
                .map(|v| if v { "Да" } else { "Нет" })
                .unwrap_or("—")
                .into(),
            Date => chrono::DateTime::from_timestamp(n as i64, 0)
                .filter(|d| d.timestamp() > 0)
                .map(|d| {
                    d.with_timezone(&chrono::Local)
                        .format("%d.%m.%Y %H:%M")
                        .to_string()
                })
                .unwrap_or_else(|| "—".into()),
            Duration => {
                if n < 0.0 || (self.key == "eta" && n >= 8_640_000.0) {
                    return "∞".into();
                }
                let seconds = n as u64;
                format!(
                    "{}д {:02}:{:02}:{:02}",
                    seconds / 86400,
                    seconds / 3600 % 24,
                    seconds / 60 % 60,
                    seconds % 60
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn widths_survive_restart_reorder_and_hidden_columns() {
        let mut settings = crate::model::Settings::default();
        settings.column_widths.insert("name".into(), 475.0);
        settings.column_widths.insert("size".into(), 82.0);
        settings.visible_columns.retain(|key| key != "size");
        move_column(&mut settings.visible_columns, "name", "upspeed", true);
        let mut restored: crate::model::Settings =
            ron::from_str(&ron::to_string(&settings).unwrap()).unwrap();
        restored.visible_columns.push("size".into());
        assert_eq!(COLUMNS[0].saved_width(&restored), 475.0);
        assert_eq!(COLUMNS[1].saved_width(&restored), 82.0);
        assert_eq!(COLUMNS[2].saved_width(&restored), COLUMNS[2].width());
        let old: crate::model::Settings = ron::from_str("()").unwrap();
        assert_eq!(COLUMNS[0].saved_width(&old), COLUMNS[0].width());
        restored.column_widths.insert("name".into(), f32::NAN);
        assert_eq!(COLUMNS[0].saved_width(&restored), COLUMNS[0].width());
    }
    #[test]
    fn reorder_in_both_directions_and_preserve_settings() {
        let mut settings = crate::model::Settings::default();
        move_column(&mut settings.visible_columns, "name", "upspeed", true);
        assert_eq!(settings.visible_columns.last().unwrap(), "name");
        move_column(&mut settings.visible_columns, "upspeed", "size", false);
        assert_eq!(settings.visible_columns.first().unwrap(), "upspeed");
        let restored: crate::model::Settings =
            ron::from_str(&ron::to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored.visible_columns, settings.visible_columns);
        let before = settings.visible_columns.clone();
        move_column(&mut settings.visible_columns, "name", "name", true);
        move_column(&mut settings.visible_columns, "name", "missing", false);
        assert_eq!(before, settings.visible_columns);
    }
    #[test]
    fn complete_unique_catalog_and_numeric_sort() {
        assert_eq!(COLUMNS.len(), 38);
        let keys: std::collections::HashSet<_> = COLUMNS.iter().map(|c| c.key).collect();
        assert_eq!(keys.len(), COLUMNS.len());
        let a: Torrent =
            serde_json::from_value(json!({"downloaded": 900, "added_on": 100})).unwrap();
        let b: Torrent =
            serde_json::from_value(json!({"downloaded": 1000, "added_on": 200})).unwrap();
        for key in ["downloaded", "added_on"] {
            assert_eq!(
                COLUMNS
                    .iter()
                    .find(|c| c.key == key)
                    .unwrap()
                    .compare(&a, &b),
                Ordering::Less
            );
        }
        assert_eq!(
            COLUMNS
                .iter()
                .find(|c| c.key == "private")
                .unwrap()
                .display(&a),
            "—"
        );
    }
    #[test]
    fn selection_survives_settings_roundtrip_and_old_settings() {
        let settings = crate::model::Settings {
            visible_columns: vec!["name".into(), "added_on".into(), "ratio".into()],
            ..Default::default()
        };
        let restored: crate::model::Settings =
            ron::from_str(&ron::to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored.visible_columns, settings.visible_columns);
        let old: crate::model::Settings = ron::from_str("()").unwrap();
        assert_eq!(old.visible_columns, defaults());
    }
}
