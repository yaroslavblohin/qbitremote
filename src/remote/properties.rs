//! Resolve aliases by API meaning, never by coincidentally equal values.
use serde_json::{Map, Value};
use std::collections::BTreeMap;
fn canonical(key: &str) -> &str {
    match key {
        "added_on" => "addition_date",
        "completion_on" => "completion_date",
        "downloaded" => "total_downloaded",
        "uploaded" => "total_uploaded",
        "downloaded_session" => "total_downloaded_session",
        "uploaded_session" => "total_uploaded_session",
        "time_active" => "time_elapsed",
        "seen_complete" => "last_seen",
        "num_complete" => "seeds_total",
        "num_incomplete" => "peers_total",
        "ratio" => "share_ratio",
        "dlspeed" => "dl_speed",
        "upspeed" => "up_speed",
        "num_seeds" => "seeds",
        "num_leechs" => "peers",
        "is_private" => "private",
        _ => key,
    }
}
pub fn split(data: &Value, extra: &BTreeMap<String, Value>) -> (Value, Value) {
    let mut main = Map::new();
    if let Some(map) = data.as_object() {
        for (key, value) in map {
            let name = canonical(key);
            if !main.contains_key(name) || main[name].is_null() || (key == name && !value.is_null())
            {
                main.insert(name.to_owned(), value.clone());
            }
        }
    }
    let mut remaining = Map::new();
    for (key, value) in extra {
        if let Some(existing) = main.get_mut(canonical(key)) {
            if existing.is_null() && !value.is_null() {
                *existing = value.clone();
            }
        } else {
            let name = canonical(key);
            if !remaining.contains_key(name)
                || remaining[name].is_null()
                || (key == name && !value.is_null())
            {
                remaining.insert(name.to_owned(), value.clone());
            }
        }
    }
    (Value::Object(main), Value::Object(remaining))
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn removes_same_fields_and_aliases_without_modifying_inputs() {
        let data = json!({"comment":"new", "addition_date":10,"completion_date":20,
            "total_downloaded":100,"total_uploaded":200,"total_downloaded_session":30,
            "total_uploaded_session":40,"time_elapsed":50,"last_seen":60,
            "seeds_total":7,"peers_total":8,"private":null,"is_private":true,"save_path":"/data"});
        let extra: BTreeMap<_,_> = serde_json::from_value(json!({"comment":"old","added_on":10,"completion_on":20,
            "downloaded":100,"uploaded":200,"downloaded_session":30,"uploaded_session":40,
            "time_active":50,"seen_complete":60,"num_complete":7,"num_incomplete":8,
            "is_private":true,"save_path":"/data","magnet_uri":"magnet:?xt=urn:btih:abc","tracker":"https://example.org/announce"})).unwrap();
        let original = extra.clone();
        let (main, remaining) = split(&data, &extra);
        assert_eq!(extra, original);
        assert_eq!(main["comment"], "new");
        assert_eq!(main["private"], true);
        assert!(main.get("is_private").is_none());
        assert_eq!(
            remaining
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["magnet_uri", "tracker"]
        );
    }
    #[test]
    fn keeps_distinct_meanings_and_missing_primary_fields() {
        let data = json!({"total_downloaded":100,"total_size":100,"seeds":7,"comment":null});
        let extra=serde_json::from_value(json!({"completed":100,"num_complete":7,"amount_left":0,"added_on":10,"comment":"fallback","unknown":100,"is_private":true,"private":null})).unwrap();
        let (main, extra) = split(&data, &extra);
        assert_eq!(main["comment"], "fallback");
        assert_eq!(extra["private"], true);
        for key in [
            "completed",
            "seeds_total",
            "amount_left",
            "addition_date",
            "unknown",
        ] {
            assert!(extra.get(key).is_some(), "{key}");
        }
        assert!(extra.get("comment").is_none());
    }
}
