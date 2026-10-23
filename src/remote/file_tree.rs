use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

#[derive(Default)]
struct Folder {
    folders: BTreeMap<String, Folder>,
    files: Vec<Value>,
    size: u64,
    done: f64,
    count: usize,
}
impl Folder {
    fn insert(&mut self, parts: &[&str], file: &Value) {
        let size = file["size"].as_u64().unwrap_or(0);
        self.size = self.size.saturating_add(size);
        self.done += size as f64 * file["progress"].as_f64().unwrap_or(0.0).clamp(0.0, 1.0);
        self.count += 1;
        if parts.len() > 1 {
            self.folders
                .entry(parts[0].into())
                .or_default()
                .insert(&parts[1..], file);
        } else {
            let mut row = file.clone();
            row["__path"] = file["name"].clone();
            row["name"] = json!(parts.first().copied().unwrap_or(""));
            self.files.push(row);
        }
    }
    fn flatten(
        &self,
        path: &str,
        depth: usize,
        collapsed: &HashSet<String>,
        searching: bool,
        out: &mut Vec<Value>,
    ) {
        for (name, folder) in &self.folders {
            let path = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}/{name}")
            };
            let open = searching || !collapsed.contains(&path);
            out.push(
                json!({"name": name, "__path": path, "__folder": true, "__depth": depth,
                "__open": open, "__count": folder.count, "size": folder.size,
                "progress": if folder.size > 0 { folder.done / folder.size as f64 } else { 0.0 }}),
            );
            if open {
                folder.flatten(&path, depth + 1, collapsed, searching, out);
            }
        }
        let mut files: Vec<_> = self.files.iter().collect();
        files.sort_by_cached_key(|file| file["name"].as_str().unwrap_or("").to_lowercase());
        for file in files {
            let mut row = file.clone();
            row["__depth"] = json!(depth);
            out.push(row);
        }
    }
}

pub fn rows(data: &Value, query: &str, collapsed: &HashSet<String>) -> Vec<Value> {
    let mut root = Folder::default();
    let query = query.trim().to_lowercase();
    for file in data.as_array().into_iter().flatten() {
        let name = file["name"].as_str().unwrap_or("");
        if !name.to_lowercase().contains(&query) {
            continue;
        }
        let parts: Vec<_> = name.split('/').filter(|s| !s.is_empty()).collect();
        root.insert(&parts, file);
    }
    let mut out = Vec::new();
    root.flatten("", 0, collapsed, !query.is_empty(), &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folders_collapse_search_reveals_and_file_identity_survives() {
        let data = json!([
            {"name":"Game/DLC/a.bin","index":7,"size":100,"progress":0.5},
            {"name":"Game/base.bin","index":2,"size":300,"progress":1.0},
            {"name":"readme.txt","index":9,"size":0,"progress":1.0}
        ]);
        let mut collapsed = HashSet::new();
        let all = rows(&data, "", &collapsed);
        assert_eq!(all.len(), 5);
        assert_eq!(all[0]["size"], 400);
        assert_eq!(all[0]["progress"], 0.875);
        collapsed.insert("Game".into());
        assert_eq!(rows(&data, "", &collapsed).len(), 2);
        let found = rows(&data, "A.BIN", &collapsed);
        assert_eq!(found.len(), 3);
        assert_eq!(found[2]["name"], "a.bin");
        assert_eq!(found[2]["__path"], "Game/DLC/a.bin");
        assert_eq!(found[2]["index"], 7);
        assert!(rows(&data, "missing", &collapsed).is_empty());
    }
}
