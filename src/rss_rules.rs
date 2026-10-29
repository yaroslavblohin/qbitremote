use serde_json::Value;
pub const MAX_BYTES: u64 = 8 * 1024 * 1024;

pub fn parse(data: &[u8]) -> Result<Value, String> {
    if data.len() as u64 > MAX_BYTES {
        return Err("Файл правил больше 8 МиБ".into());
    }
    let data = data.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(data);
    let value: Value =
        serde_json::from_slice(data).map_err(|e| format!("Некорректный JSON: {e}"))?;
    validate(&value)?;
    Ok(value)
}
pub fn validate(value: &Value) -> Result<(), String> {
    let rules = value
        .as_object()
        .ok_or("Ожидается JSON-объект: имя правила → параметры")?;
    for (name, rule) in rules {
        if name.trim().is_empty() {
            return Err("Имя правила не может быть пустым".into());
        }
        let fields = rule
            .as_object()
            .ok_or_else(|| format!("Правило «{name}»: параметры должны быть объектом"))?;
        if fields.is_empty() {
            return Err(format!("Правило «{name}» не содержит параметров"));
        }
        for (key, value) in fields {
            let valid = match key.as_str() {
                "enabled" | "useRegex" | "smartFilter" => value.is_boolean(),
                "mustContain" | "mustNotContain" | "episodeFilter" | "assignedCategory"
                | "savePath" => value.is_string(),
                "affectedFeeds" => value
                    .as_array()
                    .is_some_and(|a| a.iter().all(Value::is_string)),
                "ignoreDays" => value.as_i64().is_some_and(|i| i >= 0),
                "torrentParams" => value.is_object(),
                "addPaused" => value.is_null() || value.is_boolean(),
                _ => true,
            };
            if !valid {
                return Err(format!("Правило «{name}»: неверный тип поля {key}"));
            }
        }
    }
    Ok(())
}
pub fn read(path: &std::path::Path) -> Result<Value, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    parse(&data)
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn roundtrip_preserves_modern_legacy_and_unknown_fields() {
        let rules = json!({"Сериал":{"enabled":true,"affectedFeeds":["https://example.org/rss"],"torrentParams":{"save_path":"/data","tags":["tv"]},"futureField":{"x":1}},"Old":{"addPaused":null,"savePath":"/old"}});
        assert_eq!(
            parse(&serde_json::to_vec_pretty(&rules).unwrap()).unwrap(),
            rules
        );
    }
    #[test]
    fn rejects_invalid_documents() {
        for data in [
            "[]",
            "null",
            "{",
            "{\"x\":1}",
            "{\"\":{\"enabled\":true}}",
            "{\"x\":{\"enabled\":\"yes\"}}",
            "{\"x\":{\"affectedFeeds\":[1]}}",
        ] {
            assert!(parse(data.as_bytes()).is_err(), "{data}");
        }
    }
    #[test]
    fn accepts_bom_and_empty_export() {
        assert_eq!(parse(b"\xef\xbb\xbf{}").unwrap(), json!({}));
        assert!(parse(&vec![b' '; MAX_BYTES as usize + 1]).is_err());
    }
}
