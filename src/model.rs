use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub path_mappings: Vec<crate::path_mapping::PathMapping>,
    pub theme: crate::theme::Theme,
    pub visible_columns: Vec<String>,
    pub column_widths: std::collections::BTreeMap<String, f32>,
    pub sort_column: String,
    pub sort_descending: bool,
    pub servers: Vec<ServerProfile>,
    pub profiles_initialized: bool,
    pub selected_server: usize,
    pub server_name: String,
    pub auto_connect: bool,
    pub details_height: f32,
    pub address: String,
    pub username: String,
    pub close_to_tray: bool,
    pub zoom: f32,
    pub remember_password: bool,
    pub saved_password: Option<SavedPassword>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    #[serde(default)]
    pub path_mappings: Vec<crate::path_mapping::PathMapping>,
    pub name: String,
    pub address: String,
    pub username: String,
    pub remember_password: bool,
    pub saved_password: Option<SavedPassword>,
}

impl Settings {
    pub fn save_server(&mut self) {
        self.profiles_initialized = true;
        let profile = ServerProfile {
            path_mappings: self.path_mappings.clone(),
            name: self.server_name.clone(),
            address: self.address.clone(),
            username: self.username.clone(),
            remember_password: self.remember_password,
            saved_password: self.saved_password.clone(),
        };
        if let Some(slot) = self.servers.get_mut(self.selected_server) {
            *slot = profile;
        } else {
            self.selected_server = self.servers.len();
            self.servers.push(profile);
        }
    }

    pub fn select_server(&mut self, index: usize) {
        if let Some(profile) = self.servers.get(index).cloned() {
            self.selected_server = index;
            self.path_mappings = profile.path_mappings;
            self.server_name = profile.name;
            self.address = profile.address;
            self.username = profile.username;
            self.remember_password = profile.remember_password;
            self.saved_password = profile.saved_password;
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SavedPassword {
    pub identity: String,
    pub encrypted: Vec<u8>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            path_mappings: Vec::new(),
            theme: Default::default(),
            visible_columns: crate::columns::defaults(),
            column_widths: Default::default(),
            sort_column: "name".into(),
            sort_descending: false,
            servers: Vec::new(),
            profiles_initialized: false,
            selected_server: 0,
            server_name: "Основной сервер".into(),
            auto_connect: true,
            details_height: 280.0,
            address: "http://localhost:8080".into(),
            username: "admin".into(),
            close_to_tray: true,
            zoom: 1.0,
            remember_password: true,
            saved_password: None,
        }
    }
}

#[derive(Clone, Default, Deserialize)]
#[serde(default)]
pub struct Torrent {
    #[serde(flatten)]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
    pub hash: String,
    pub name: String,
    pub size: u64,
    pub progress: f32,
    pub dlspeed: u64,
    pub upspeed: u64,
    pub state: String,
    pub category: String,
    pub save_path: String,
    pub num_seeds: u32,
    pub num_leechs: u32,
    pub ratio: f64,
    pub eta: i64,
}

impl Torrent {
    pub fn stopped(&self) -> bool {
        self.state.starts_with("paused") || self.state.starts_with("stopped")
    }
    pub fn downloading(&self) -> bool {
        !self.stopped() && self.progress < 1.0
    }
    pub fn status(&self) -> &str {
        match self.state.as_str() {
            "downloading" => "Загрузка",
            "uploading" => "Раздача",
            "stalledDL" => "Ожидание пиров",
            "stalledUP" => "Раздача · ожидание",
            "pausedDL" | "pausedUP" | "stoppedDL" | "stoppedUP" => "Остановлен",
            "error" => "Ошибка",
            "missingFiles" => "Нет файлов",
            "metaDL" | "forcedMetaDL" => "Метаданные",
            "checkingDL" | "checkingUP" | "checkingResumeData" => "Проверка",
            "queuedDL" | "queuedUP" => "В очереди",
            "forcedDL" => "Загрузка принудительно",
            "forcedUP" => "Раздача принудительно",
            _ => &self.state,
        }
    }
}

pub fn bytes(value: u64) -> String {
    let mut n = value as f64;
    let units = ["Б", "КиБ", "МиБ", "ГиБ", "ТиБ"];
    let mut i = 0;
    while n >= 1024.0 && i < units.len() - 1 {
        n /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{value} Б")
    } else {
        format!("{n:.1} {}", units[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn path_mappings_survive_serialization_and_remain_per_server() {
        let mut settings = Settings::default();
        settings
            .path_mappings
            .push(crate::path_mapping::PathMapping {
                server: "/downloads".into(),
                local: "Z:\\Downloads".into(),
            });
        settings.save_server();
        settings.selected_server = 1;
        settings.server_name = "Второй".into();
        settings.path_mappings.clear();
        settings.save_server();
        let encoded = ron::to_string(&settings).unwrap();
        let mut decoded: Settings = ron::from_str(&encoded).unwrap();
        decoded.select_server(0);
        assert_eq!(decoded.path_mappings[0].local, "Z:\\Downloads");
        decoded.path_mappings[0].local = "Y:\\".into();
        decoded.save_server();
        decoded.select_server(1);
        assert!(decoded.path_mappings.is_empty());
        decoded.select_server(0);
        assert_eq!(decoded.path_mappings[0].local, "Y:\\");
    }
    #[test]
    fn old_server_profile_loads_without_mappings() {
        let profile: ServerProfile = ron::from_str(
            "(name:\"Old\",address:\"http://host:8080\",username:\"admin\",remember_password:false,saved_password:None)"
        ).unwrap();
        assert!(profile.path_mappings.is_empty());
    }
    #[test]
    fn stopped_states_across_api_versions() {
        for state in ["pausedDL", "pausedUP", "stoppedDL", "stoppedUP"] {
            let t = Torrent {
                state: state.into(),
                ..Default::default()
            };
            assert!(t.stopped());
            assert!(!t.downloading());
        }
    }
    #[test]
    fn binary_units() {
        assert_eq!(bytes(1024), "1.0 КиБ");
        assert_eq!(bytes(0), "0 Б");
    }
}
