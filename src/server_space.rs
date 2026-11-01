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
pub fn color(free: Option<u64>, palette: &crate::theme::Palette) -> egui::Color32 {
    match level(free) {
        Level::Unknown => palette.muted,
        Level::Normal => palette.text,
        Level::Low => palette.orange,
        Level::Critical => palette.red,
    }
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
