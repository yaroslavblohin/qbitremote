use serde_json::Value;

pub(super) struct Row<'a> {
    pub data: &'a Value,
    pub kind: Kind,
}

#[derive(PartialEq, Eq)]
pub(super) enum Kind {
    Tracker,
    Endpoint,
}

pub(super) fn endpoints(tracker: &Value) -> &[Value] {
    tracker["endpoints"].as_array().map_or(&[], Vec::as_slice)
}

pub(super) fn can_expand(tracker: &Value) -> bool {
    !endpoints(tracker).is_empty()
        && tracker["url"]
            .as_str()
            .is_some_and(|url| url.contains("://"))
}

pub(super) fn visible_rows(trackers: &[Value], expanded: impl Fn(&str) -> bool) -> Vec<Row<'_>> {
    let mut rows = Vec::with_capacity(trackers.len());
    for tracker in trackers {
        rows.push(Row {
            data: tracker,
            kind: Kind::Tracker,
        });
        if can_expand(tracker) && tracker["url"].as_str().is_some_and(&expanded) {
            rows.extend(endpoints(tracker).iter().map(|endpoint| Row {
                data: endpoint,
                kind: Kind::Endpoint,
            }));
        }
    }
    rows
}

pub(super) fn status(
    data: &Value,
    palette: crate::theme::Palette,
) -> (&'static str, eframe::egui::Color32) {
    if data["updating"].as_bool() == Some(true) {
        return ("Обновляется", palette.blue);
    }
    match data["status"].as_i64() {
        Some(0) => ("Отключён", palette.muted),
        Some(1) => ("Не обращались", palette.muted),
        Some(2) => ("Работает", palette.green),
        Some(3) => ("Обновляется", palette.blue),
        Some(4) => ("Не работает", palette.red),
        Some(5) => ("Ошибка трекера", palette.red),
        Some(6) => ("Нет доступа", palette.red),
        _ => ("Неизвестно", palette.muted),
    }
}

pub(super) fn announce_time(data: &Value, key: &str, now: i64) -> String {
    let Some(deadline) = data[key].as_i64().filter(|value| *value > 0) else {
        return "—".into();
    };
    let seconds = deadline.saturating_sub(now).max(0);
    if seconds >= 86_400 {
        format!("{} д {} ч", seconds / 86_400, seconds / 3_600 % 24)
    } else if seconds >= 3_600 {
        format!("{} ч {} мин", seconds / 3_600, seconds / 60 % 60)
    } else if seconds >= 60 {
        format!("{} мин {} с", seconds / 60, seconds % 60)
    } else {
        format!("{seconds} с")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn modern_endpoints_appear_under_expanded_tracker_and_legacy_remains_flat() {
        let data = json!([
            {"url":"https://tracker.example/announce","endpoints":[
                {"name":"192.0.2.10:51413","bt_version":1},
                {"name":"[2001:db8::10]:51413","bt_version":2}
            ]},
            {"url":"udp://legacy.example:6969/announce","status":2},
            {"url":"http://waiting.example/announce","status":1,"endpoints":[]}
        ]);
        let trackers = data.as_array().unwrap();
        assert_eq!(visible_rows(trackers, |_| false).len(), 3);
        let visible = visible_rows(trackers, |_| true);
        assert_eq!(visible.len(), 5);
        assert!(visible[1].kind == Kind::Endpoint && visible[2].kind == Kind::Endpoint);
        assert_eq!(visible[2].data["name"], "[2001:db8::10]:51413");
        assert_eq!(visible[2].data["bt_version"], 2);
        assert!(visible[3].kind == Kind::Tracker && !can_expand(visible[3].data));
        assert!(visible[4].kind == Kind::Tracker && !can_expand(visible[4].data));
    }

    #[test]
    fn timestamps_are_countdowns_and_updating_overrides_status() {
        let tracker = json!({"next_announce": 1_700_000_125_i64, "min_announce": 0, "status": 6, "updating": true});
        assert_eq!(
            announce_time(&tracker, "next_announce", 1_700_000_000),
            "2 мин 5 с"
        );
        assert_eq!(
            announce_time(&tracker, "next_announce", 1_700_000_200),
            "0 с"
        );
        assert_eq!(announce_time(&tracker, "min_announce", 1_700_000_000), "—");
        let palette = crate::theme::Theme::Graphite.palette();
        assert_eq!(status(&tracker, palette).0, "Обновляется");
        assert_eq!(status(&json!({"status": 6}), palette).0, "Нет доступа");
    }
}
