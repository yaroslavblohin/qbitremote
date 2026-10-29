use crate::{
    api::Command,
    model::{bytes, Torrent},
};
use eframe::egui::{self, RichText};
use egui_extras::{Column, TableBuilder};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, VecDeque},
    time::{Duration, Instant},
};
mod file_tree;
mod pages;

fn menu_group(ui: &mut egui::Ui, title: &str) {
    let p = crate::theme::palette(ui.ctx());
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 17.0), egui::Sense::hover());
    ui.painter().line_segment(
        [rect.left_top(), rect.right_top()],
        egui::Stroke::new(1.0_f32, p.border),
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(3.0, 2.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(10.0),
        p.muted,
    );
}

fn action_menu_item(ui: &mut egui::Ui, label: &str, icon: usize, checked: bool) -> egui::Response {
    let p = crate::theme::palette(ui.ctx());
    let font = egui::FontId::proportional(12.0);
    let width = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font.clone(), p.text)
        .size()
        .x;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(width + 48.0).max(270.0), 19.0),
        egui::Sense::click(),
    );
    if response.hovered() || response.has_focus() {
        ui.painter().rect_filled(rect, 4.0, p.hover);
    }
    crate::app::draw_icon(
        ui.painter(),
        rect.left_center() + egui::vec2(11.0, 0.0),
        icon,
        p.accent,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(26.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        font.clone(),
        p.text,
    );
    if checked {
        ui.painter().text(
            rect.right_center() - egui::vec2(14.0, 0.0),
            egui::Align2::CENTER_CENTER,
            "✓",
            font,
            p.green,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Torrents,
    Speed,
    Search,
    Rss,
    Log,
    Preferences,
    Categories,
}
pub const PAGES: [(Page, &str); 7] = [
    (Page::Torrents, "Торренты"),
    (Page::Speed, "Скорость и статистика"),
    (Page::Search, "Поиск в сети"),
    (Page::Rss, "RSS"),
    (Page::Log, "Журнал"),
    (Page::Preferences, "Настройки сервера"),
    (Page::Categories, "Категории и теги"),
];

struct Cached {
    data: Result<Value, String>,
    at: Instant,
}
struct Pending {
    key: String,
    post: bool,
}
pub struct Field {
    pub key: String,
    pub label: String,
    pub value: String,
}
impl Field {
    fn new(key: &str, label: &str, value: impl ToString) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            value: value.to_string(),
        }
    }
}
struct Form {
    title: String,
    path: String,
    fixed: Vec<(String, String)>,
    fields: Vec<Field>,
    warning: String,
}

pub struct Remote {
    pub page: Page,
    tab: usize,
    cache: HashMap<String, Cached>,
    pending: HashMap<u64, Pending>,
    serial: u64,
    queue: Vec<Command>,
    form: Option<Form>,
    pub message: String,
    pub error: bool,
    selected_hash: String,
    file_filter: String,
    collapsed_folders: std::collections::HashSet<String>,
    search_pattern: String,
    search_category: String,
    search_plugins: String,
    search_id: Option<i64>,
    search_offset: usize,
    prefs: Option<Value>,
    prefs_original: Option<Value>,
    prefs_filter: String,
    rss_rules: bool,
    log_peers: bool,
    log_filter: String,
    history: VecDeque<(f32, f32)>,
    torrent_history: VecDeque<(f32, f32)>,
}
impl Default for Remote {
    fn default() -> Self {
        Self {
            page: Page::Torrents,
            tab: 0,
            cache: HashMap::new(),
            pending: HashMap::new(),
            serial: 0,
            queue: vec![],
            form: None,
            message: String::new(),
            error: false,
            selected_hash: String::new(),
            file_filter: String::new(),
            collapsed_folders: Default::default(),
            search_pattern: String::new(),
            search_category: "all".into(),
            search_plugins: "enabled".into(),
            search_id: None,
            search_offset: 0,
            prefs: None,
            prefs_original: None,
            prefs_filter: String::new(),
            rss_rules: false,
            log_peers: false,
            log_filter: String::new(),
            history: VecDeque::new(),
            torrent_history: VecDeque::new(),
        }
    }
}
impl Remote {
    pub fn dialog_open(&self) -> bool {
        self.form.is_some()
    }
    #[cfg(feature = "ui-smoke")]
    pub fn smoke_fixture(&mut self) {
        if std::env::var("QBIT_SMOKE_PAGE").as_deref() == Ok("files") {
            self.tab = 4;
        }
        self.page = match std::env::var("QBIT_SMOKE_PAGE").as_deref() {
            Ok("speed") => Page::Speed,
            Ok("settings") => Page::Preferences,
            _ => Page::Torrents,
        };
        for (key, value) in [
            (
                "files/fixture-1",
                json!([
                    {"name":"Linux/Images/ubuntu.iso", "index":0, "size":6000000000_u64, "progress":0.64, "priority":1, "availability":1.2},
                    {"name":"Linux/Docs/install.txt", "index":1, "size":12000, "progress":1.0, "priority":1, "availability":2.0},
                    {"name":"Linux/Docs/license.txt", "index":2, "size":4000, "progress":1.0, "priority":0, "availability":2.0},
                    {"name":"readme.txt", "index":3, "size":8000, "progress":1.0, "priority":1, "availability":1.0}
                ]),
            ),
            (
                "properties/fixture-1",
                json!({"save_path":"/downloads/Linux","total_size":6000000000_u64,"total_downloaded":3840000000_u64,"total_uploaded":23000000,"dl_speed":12450000,"up_speed":120000,"dl_limit":0,"up_limit":1048576,"seeds":24,"peers":8,"pieces_num":1500,"pieces_have":960,"share_ratio":0.006,"comment":"Тестовые данные"}),
            ),
            (
                "transfer",
                json!({"dl_info_speed":12450000,"up_info_speed":2100000,"dl_rate_limit":0,"up_rate_limit":10485760,"dht_nodes":320,"connection_status":"connected"}),
            ),
            ("speed-mode", json!(0)),
            ("build", json!({"qt":"6.x","libtorrent":"2.x"})),
            (
                "preferences",
                json!({"dl_limit":0,"up_limit":10485760,"alt_dl_limit":1048576,"alt_up_limit":524288,"scheduler_enabled":false,"save_path":"/downloads","dht":true,"pex":true}),
            ),
        ] {
            self.cache.insert(
                key.into(),
                Cached {
                    data: Ok(value.clone()),
                    at: Instant::now(),
                },
            );
            if key == "preferences" {
                self.prefs = Some(value.clone());
                self.prefs_original = Some(value);
            }
        }
        for i in 0..60 {
            push_history(
                &mut self.history,
                8_000_000.0 + (i as f32 * 0.3).sin() * 3_000_000.0,
                2_000_000.0,
            );
        }
    }
    pub fn clear(&mut self) {
        self.cache.clear();
        self.pending.clear();
        self.queue.clear();
        self.form = None;
        self.prefs = None;
        self.prefs_original = None;
        self.search_id = None;
        self.message.clear();
        self.history.clear();
        self.torrent_history.clear();
        self.selected_hash.clear();
        self.collapsed_folders.clear();
    }
    pub fn commands(&mut self) -> Vec<Command> {
        std::mem::take(&mut self.queue)
    }
    fn request(&mut self, key: &str, path: &str, fields: Vec<(String, String)>, post: bool) {
        if !post && self.pending.values().any(|p| p.key == key) {
            return;
        }
        self.serial += 1;
        self.pending.insert(
            self.serial,
            Pending {
                key: key.into(),
                post,
            },
        );
        self.queue.push(Command::Remote {
            token: self.serial,
            path: path.into(),
            fields,
            post,
        });
    }
    fn get(&mut self, key: &str, path: &str, fields: Vec<(String, String)>) -> Option<Value> {
        if self
            .cache
            .get(key)
            .is_none_or(|c| c.at.elapsed() > Duration::from_secs(5))
        {
            self.request(key, path, fields, false);
        }
        self.cache
            .get(key)
            .and_then(|c| c.data.as_ref().ok())
            .cloned()
    }
    fn action(&mut self, title: &str, path: &str, fields: Vec<(String, String)>) {
        self.request(title, path, fields, true);
        self.message = format!("{title}: выполнение…");
        self.error = false;
    }
    pub fn reply(&mut self, token: u64, result: Result<Value, String>) {
        let Some(pending) = self.pending.remove(&token) else {
            return;
        };
        if pending.post {
            match result {
                Ok(value) => {
                    if pending.key == "Начать поиск" {
                        self.search_id = value["id"].as_i64();
                        self.search_offset = 0;
                    }
                    if pending.key == "Сохранить настройки" {
                        self.prefs = None;
                        self.prefs_original = None;
                    }
                    self.message = format!("{}: выполнено", pending.key);
                    self.error = false;
                }
                Err(error) => {
                    self.message = error;
                    self.error = true;
                }
            }
            self.cache.clear();
        } else {
            if let Ok(value) = &result {
                if pending.key == "transfer" {
                    push_history(
                        &mut self.history,
                        value["dl_info_speed"].as_f64().unwrap_or(0.0) as f32,
                        value["up_info_speed"].as_f64().unwrap_or(0.0) as f32,
                    );
                }
                if pending.key == format!("properties/{}", self.selected_hash) {
                    push_history(
                        &mut self.torrent_history,
                        value["dl_speed"].as_f64().unwrap_or(0.0) as f32,
                        value["up_speed"].as_f64().unwrap_or(0.0) as f32,
                    );
                }
                if pending.key == "preferences" && self.prefs.is_none() {
                    self.prefs = Some(value.clone());
                    self.prefs_original = Some(value.clone());
                }
            }
            self.cache.insert(
                pending.key,
                Cached {
                    data: result,
                    at: Instant::now(),
                },
            );
        }
    }
    fn loading(&self, ui: &mut egui::Ui, key: &str) {
        if let Some(Cached {
            data: Err(error), ..
        }) = self.cache.get(key)
        {
            ui.colored_label(crate::theme::palette(ui.ctx()).red, error);
        } else {
            ui.spinner();
            ui.label("Загрузка с сервера…");
        }
    }
    fn open_form(
        &mut self,
        title: &str,
        path: &str,
        fixed: Vec<(String, String)>,
        fields: Vec<Field>,
        warning: &str,
    ) {
        self.form = Some(Form {
            title: title.into(),
            path: path.into(),
            fixed,
            fields,
            warning: warning.into(),
        });
    }
    pub fn dialogs(&mut self, ctx: &egui::Context, connected: bool) {
        let Some(mut form) = self.form.take() else {
            return;
        };
        let mut open = true;
        let mut submit = false;
        egui::Window::new(&form.title)
            .id(egui::Id::new("remote-action"))
            .open(&mut open)
            .collapsible(false)
            .vscroll(true)
            .default_width(480.0)
            .show(ctx, |ui| {
                if !form.warning.is_empty() {
                    ui.label(&form.warning);
                    ui.separator();
                }
                for field in &mut form.fields {
                    ui.label(&field.label);
                    if field.key == "ruleDef" {
                        if let Ok(mut value) = serde_json::from_str::<Value>(&field.value) {
                            pages::edit_value(ui, "rule", &mut value);
                            field.value = value.to_string();
                        }
                    } else if field.value == "true" || field.value == "false" {
                        let mut b = field.value == "true";
                        if ui.checkbox(&mut b, "Включено").changed() {
                            field.value = b.to_string();
                        }
                    } else if field.key == "urls" || field.key == "peers" || field.key == "comment"
                    {
                        ui.add(
                            egui::TextEdit::multiline(&mut field.value)
                                .desired_rows(3)
                                .desired_width(f32::INFINITY),
                        );
                    } else {
                        ui.add(
                            egui::TextEdit::singleline(&mut field.value)
                                .desired_width(f32::INFINITY),
                        );
                    }
                }
                submit = ui
                    .add_enabled(connected, egui::Button::new("Применить"))
                    .clicked();
            });
        if submit {
            let mut fields = form.fixed.clone();
            for field in &form.fields {
                let value = if field.key == "limit" {
                    match kib_to_bytes(&field.value) {
                        Ok(v) => v.to_string(),
                        Err(e) => {
                            self.message = e;
                            self.error = true;
                            self.form = Some(form);
                            return;
                        }
                    }
                } else {
                    field.value.clone()
                };
                fields.push((field.key.clone(), value));
            }
            self.action(&form.title, &form.path, fields);
        } else if open {
            self.form = Some(form);
        }
    }
    pub fn details(&mut self, ui: &mut egui::Ui, torrent: &Torrent) {
        let hash = &torrent.hash;
        if self.selected_hash != *hash {
            self.selected_hash = hash.clone();
            self.file_filter.clear();
            self.collapsed_folders.clear();
            self.torrent_history.clear();
        }
        ui.horizontal_wrapped(|ui| {
            for (i, name) in [
                "Общее",
                "Трекеры",
                "Пиры",
                "HTTP-источники",
                "Файлы",
                "Части",
                "График",
            ]
            .iter()
            .enumerate()
            {
                ui.selectable_value(&mut self.tab, i, *name);
            }
        });
        ui.separator();
        let endpoint = match self.tab {
            1 => "torrents/trackers",
            2 => "sync/torrentPeers",
            3 => "torrents/webseeds",
            4 => "torrents/files",
            5 => "torrents/pieceStates",
            _ => "torrents/properties",
        };
        let key = format!("{}/{}", endpoint.split('/').next_back().unwrap(), hash);
        let mut args = vec![("hash".into(), hash.clone())];
        if self.tab == 2 {
            args.push(("rid".into(), "0".into()));
        }
        let Some(data) = self.get(&key, endpoint, args) else {
            self.loading(ui, &key);
            return;
        };
        match self.tab {
            0 => {
                ui.horizontal_wrapped(|ui| {
                    self.limit_button(ui, Some(hash), true, number(&data, "dl_limit"));
                    self.limit_button(ui, Some(hash), false, number(&data, "up_limit"));
                    if ui.button("Скопировать magnet").clicked() {
                        ui.ctx().copy_text(
                            torrent
                                .extra
                                .get("magnet_uri")
                                .and_then(Value::as_str)
                                .map(str::to_owned)
                                .unwrap_or_else(|| format!("magnet:?xt=urn:btih:{hash}")),
                        );
                    }
                });
                egui::ScrollArea::vertical()
                    .id_salt("general")
                    .show(ui, |ui| {
                        object_grid(ui, "properties", &data);
                        ui.collapsing("Дополнительные сведения", |ui| {
                            let value = serde_json::to_value(&torrent.extra).unwrap_or_default();
                            object_grid(ui, "extra", &value);
                        });
                    });
            }
            1 => self.trackers(ui, hash, &data),
            2 => self.peers(ui, hash, &data),
            3 => self.webseeds(ui, hash, &data),
            4 => self.files(ui, hash, &data),
            5 => {
                ui.label("Серый — отсутствует, синий — загружается, светлый — готов");
                let pieces = data.as_array().cloned().unwrap_or_default();
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 60.0),
                    egui::Sense::hover(),
                );
                let bins = pieces.len().min(rect.width() as usize).max(1);
                for i in 0..bins {
                    let index = i * pieces.len() / bins;
                    let state = pieces.get(index).and_then(Value::as_i64).unwrap_or(0);
                    let color = match state {
                        2 => crate::theme::palette(ui.ctx()).muted,
                        1 => crate::theme::palette(ui.ctx()).blue,
                        _ => crate::theme::palette(ui.ctx()).button,
                    };
                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(
                            rect.min + egui::vec2(i as f32 * rect.width() / bins as f32, 0.0),
                            rect.min
                                + egui::vec2((i + 1) as f32 * rect.width() / bins as f32, 60.0),
                        ),
                        0.0,
                        color,
                    );
                }
                ui.label(format!(
                    "Частей: {} · готово: {}",
                    pieces.len(),
                    pieces.iter().filter(|p| p.as_i64() == Some(2)).count()
                ));
            }
            _ => speed_graph(ui, &self.torrent_history),
        }
    }
    fn limit_button(
        &mut self,
        ui: &mut egui::Ui,
        hash: Option<&String>,
        download: bool,
        current: i64,
    ) {
        let title = if download {
            "Лимит скачивания"
        } else {
            "Лимит отдачи"
        };
        let label = if current <= 0 {
            "без ограничения".into()
        } else {
            format!("{}/с", bytes(current as u64))
        };
        if ui.button(format!("{title}: {label}")).clicked() {
            let group = if hash.is_some() {
                "torrents"
            } else {
                "transfer"
            };
            let method = if download {
                "setDownloadLimit"
            } else {
                "setUploadLimit"
            };
            self.open_form(
                title,
                &format!("{group}/{method}"),
                hash.map(|h| vec![("hashes".into(), h.clone())])
                    .unwrap_or_default(),
                vec![Field::new(
                    "limit",
                    "КиБ/с · 0 = без ограничения",
                    (current.max(0) as f64 / 1024.0).to_string(),
                )],
                if hash.is_some() {
                    "Ограничение выбранного торрента."
                } else {
                    "Ограничение для всех загрузок на удалённом сервере (текущий режим скорости)."
                },
            );
        }
    }
    pub fn torrent_actions(&mut self, ui: &mut egui::Ui, t: &Torrent, hashes: &str, count: usize) {
        ui.set_width(290.0);
        ui.spacing_mut().item_spacing.y = 1.0;
        egui::ScrollArea::vertical()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .max_height((ui.ctx().screen_rect().height() - 80.0).max(180.0))
            .show(ui, |ui| {
                menu_group(ui, "Проверка и трекеры");
                for (label, method) in [
                    ("Перепроверить", "recheck"),
                    ("Обновить трекеры", "reannounce"),
                    ("Выше в очереди", "increasePrio"),
                    ("Ниже в очереди", "decreasePrio"),
                    ("В начало очереди", "topPrio"),
                    ("В конец очереди", "bottomPrio"),
                    ("Последовательная загрузка", "toggleSequentialDownload"),
                    ("Приоритет крайних частей", "toggleFirstLastPiecePrio"),
                ] {
                    if method == "increasePrio" {
                        menu_group(ui, "Очередь");
                    }
                    if method == "toggleSequentialDownload" {
                        menu_group(ui, "Режим загрузки");
                    }
                    let icon = match method {
                        "recheck" => 6,
                        "reannounce" => 7,
                        "increasePrio" | "topPrio" => 2,
                        "decreasePrio" | "bottomPrio" => 1,
                        _ => 8,
                    };
                    if action_menu_item(ui, label, icon, false).clicked() {
                        self.action(
                            label,
                            &format!("torrents/{method}"),
                            vec![("hashes".into(), hashes.to_owned())],
                        );
                        ui.close_menu();
                    }
                }
                menu_group(ui, "Управление и раздача");
                for (label, method, key) in [
                    ("Принудительный запуск", "setForceStart", "force_start"),
                    ("Автоуправление", "setAutoManagement", "auto_tmm"),
                    ("Суперсидирование", "setSuperSeeding", "super_seeding"),
                ] {
                    if count > 1 {
                        ui.menu_button(label, |ui| {
                            for (caption, value) in [
                                ("Включить для выбранных", true),
                                ("Выключить для выбранных", false),
                            ] {
                                if ui.button(caption).clicked() {
                                    self.action(
                                        label,
                                        &format!("torrents/{method}"),
                                        vec![
                                            ("hashes".into(), hashes.into()),
                                            ("enable".into(), value.to_string()),
                                            ("value".into(), value.to_string()),
                                        ],
                                    );
                                    ui.close_menu();
                                }
                            }
                        });
                        continue;
                    }
                    let enabled =
                        count == 1 && t.extra.get(key).and_then(Value::as_bool).unwrap_or(false);
                    if action_menu_item(ui, label, 9, enabled).clicked() {
                        self.action(
                            label,
                            &format!("torrents/{method}"),
                            vec![
                                ("hashes".into(), hashes.to_owned()),
                                ("enable".into(), (!enabled).to_string()),
                                ("value".into(), (!enabled).to_string()),
                            ],
                        );
                        ui.close_menu();
                    }
                }
                menu_group(ui, "Данные и организация");
                for (title, path, key, label, initial) in [
                    (
                        "Переименовать торрент",
                        "rename",
                        "name",
                        "Имя",
                        t.name.clone(),
                    ),
                    (
                        "Изменить категорию",
                        "setCategory",
                        "category",
                        "Категория (пусто — убрать)",
                        t.category.clone(),
                    ),
                    (
                        "Добавить теги",
                        "addTags",
                        "tags",
                        "Теги через запятую",
                        String::new(),
                    ),
                    (
                        "Удалить теги",
                        "removeTags",
                        "tags",
                        "Теги через запятую",
                        String::new(),
                    ),
                    (
                        "Изменить комментарий",
                        "setComment",
                        "comment",
                        "Комментарий",
                        String::new(),
                    ),
                    (
                        "Переместить данные на сервере",
                        "setLocation",
                        "location",
                        "Новый путь на сервере",
                        t.save_path.clone(),
                    ),
                ] {
                    if path == "rename" && count > 1 {
                        continue;
                    }
                    if action_menu_item(
                        ui,
                        title,
                        if path == "setLocation" { 2 } else { 10 },
                        false,
                    )
                    .clicked()
                    {
                        let hk = if path == "rename" { "hash" } else { "hashes" };
                        self.open_form(
                            title,
                            &format!("torrents/{path}"),
                            vec![(
                                hk.into(),
                                if path == "rename" {
                                    t.hash.clone()
                                } else {
                                    hashes.to_owned()
                                },
                            )],
                            vec![Field::new(key, label, initial)],
                            "Изменение применяется на удалённом сервере.",
                        );
                        ui.close_menu();
                    }
                }
                menu_group(ui, "Ограничения и экспорт");
                for (title, method) in [
                    ("Лимит скачивания…", "setDownloadLimit"),
                    ("Лимит отдачи…", "setUploadLimit"),
                ] {
                    if action_menu_item(ui, title, 9, false).clicked() {
                        self.open_form(
                            title,
                            &format!("torrents/{method}"),
                            vec![("hashes".into(), hashes.into())],
                            vec![Field::new("limit", "КиБ/с · 0 без ограничения", 0)],
                            &format!("Применить к выбранным торрентам: {count}"),
                        );
                        ui.close_menu();
                    }
                }
                if action_menu_item(ui, "Ограничения раздачи…", 9, false).clicked()
                {
                    self.open_form(
                        "Ограничения раздачи",
                        "torrents/setShareLimits",
                        vec![
                            ("hashes".into(), hashes.to_owned()),
                            ("shareLimitAction".into(), "Default".into()),
                            ("shareLimitsMode".into(), "Default".into()),
                        ],
                        vec![
                            Field::new(
                                "ratioLimit",
                                "Рейтинг · -2 по умолчанию, -1 без ограничения",
                                -2,
                            ),
                            Field::new(
                                "seedingTimeLimit",
                                "Время раздачи, минуты · -2 по умолчанию, -1 без ограничения",
                                -2,
                            ),
                            Field::new(
                                "inactiveSeedingTimeLimit",
                                "Неактивная раздача, минуты · -2 по умолчанию, -1 без ограничения",
                                -2,
                            ),
                        ],
                        "",
                    );
                    ui.close_menu();
                }
                if count == 1
                    && action_menu_item(ui, "Экспортировать .torrent…", 1, false).clicked()
                {
                    if let Some(destination) = rfd::FileDialog::new()
                        .add_filter("Torrent", &["torrent"])
                        .set_file_name("export.torrent")
                        .save_file()
                    {
                        self.serial += 1;
                        self.pending.insert(
                            self.serial,
                            Pending {
                                key: "Экспорт торрента".into(),
                                post: true,
                            },
                        );
                        self.queue.push(Command::Export {
                            token: self.serial,
                            hash: t.hash.clone(),
                            destination,
                        });
                    }
                    ui.close_menu();
                }
            });
    }
    fn trackers(&mut self, ui: &mut egui::Ui, hash: &str, data: &Value) {
        ui.horizontal(|ui| {
            if ui.button("Добавить трекеры").clicked() {
                self.open_form(
                    "Добавить трекеры",
                    "torrents/addTrackers",
                    vec![("hash".into(), hash.into())],
                    vec![Field::new("urls", "URL, по одному на строку", "")],
                    "",
                );
            }
            if ui.button("Обновить анонс").clicked() {
                self.action(
                    "Обновить анонс",
                    "torrents/reannounce",
                    vec![("hashes".into(), hash.into())],
                );
            }
        });
        egui::ScrollArea::both().id_salt("trackers").show(ui, |ui| {
            for tracker in data.as_array().into_iter().flatten() {
                let url = text(tracker, "url");
                ui.group(|ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&url).strong());
                        ui.label(match number(tracker, "status") {
                            0 => "Отключён",
                            1 => "Не обращались",
                            2 => "Работает",
                            3 => "Обновляется",
                            4 => "Ошибка",
                            _ => "Неизвестно",
                        });
                        if url.contains("://") {
                            if ui.small_button("Изменить").clicked() {
                                self.open_form(
                                    "Изменить трекер",
                                    "torrents/editTracker",
                                    vec![
                                        ("hash".into(), hash.into()),
                                        ("origUrl".into(), url.clone()),
                                    ],
                                    vec![Field::new("newUrl", "Новый URL", &url)],
                                    "",
                                );
                            }
                            if ui.small_button("Удалить").clicked() {
                                self.open_form(
                                    "Удалить трекер",
                                    "torrents/removeTrackers",
                                    vec![
                                        ("hash".into(), hash.into()),
                                        ("urls".into(), url.clone()),
                                    ],
                                    vec![],
                                    &url,
                                );
                            }
                        }
                    });
                    ui.label(format!(
                        "Уровень {} · сиды {} · личи {} · пиры {} · скачан {}",
                        number(tracker, "tier"),
                        number(tracker, "num_seeds"),
                        number(tracker, "num_leeches"),
                        number(tracker, "num_peers"),
                        number(tracker, "num_downloaded")
                    ));
                    ui.label(text(tracker, "msg"));
                });
            }
        });
    }
    fn peers(&mut self, ui: &mut egui::Ui, hash: &str, data: &Value) {
        if ui.button("Добавить пиров").clicked() {
            self.open_form(
                "Добавить пиров",
                "torrents/addPeers",
                vec![("hashes".into(), hash.into())],
                vec![Field::new(
                    "peers",
                    "IP:порт через | (IPv6: [адрес]:порт)",
                    "",
                )],
                "",
            );
        }
        let rows: Vec<Value> = data["peers"]
            .as_object()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default();
        data_table(
            ui,
            "peers",
            &rows,
            &[
                ("ip", "IP"),
                ("port", "Порт"),
                ("country", "Страна"),
                ("connection", "Соединение"),
                ("client", "Клиент"),
                ("progress", "Прогресс"),
                ("dl_speed", "Скачивание"),
                ("up_speed", "Отдача"),
                ("downloaded", "Скачано"),
                ("uploaded", "Отдано"),
                ("flags", "Флаги"),
            ],
            |ui, row| {
                if ui.small_button("Заблокировать").clicked() {
                    self.open_form(
                        "Заблокировать пира",
                        "transfer/banPeers",
                        vec![(
                            "peers".into(),
                            peer_address(&text(row, "ip"), number(row, "port")),
                        )],
                        vec![],
                        "Пир будет заблокирован на сервере.",
                    );
                }
            },
        );
    }
    fn webseeds(&mut self, ui: &mut egui::Ui, hash: &str, data: &Value) {
        if ui.button("Добавить HTTP-источники").clicked() {
            self.open_form(
                "Добавить HTTP-источники",
                "torrents/addWebSeeds",
                vec![("hash".into(), hash.into())],
                vec![Field::new("urls", "URL через |", "")],
                "",
            );
        }
        for seed in data.as_array().into_iter().flatten() {
            let url = text(seed, "url");
            ui.horizontal_wrapped(|ui| {
                ui.label(&url);
                if ui.button("Изменить").clicked() {
                    self.open_form(
                        "Изменить HTTP-источник",
                        "torrents/editWebSeed",
                        vec![
                            ("hash".into(), hash.into()),
                            ("origUrl".into(), url.clone()),
                        ],
                        vec![Field::new("newUrl", "Новый URL", &url)],
                        "",
                    );
                }
                if ui.button("Удалить").clicked() {
                    self.open_form(
                        "Удалить HTTP-источник",
                        "torrents/removeWebSeeds",
                        vec![("hash".into(), hash.into()), ("urls".into(), url.clone())],
                        vec![],
                        &url,
                    );
                }
            });
        }
    }
    fn files(&mut self, ui: &mut egui::Ui, hash: &str, data: &Value) {
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.file_filter)
                    .hint_text("Поиск по имени или пути…"),
            );
            if ui.button("Развернуть всё").clicked() {
                self.collapsed_folders.clear();
            }
            if ui
                .add_enabled(
                    self.file_filter.trim().is_empty(),
                    egui::Button::new("Свернуть всё"),
                )
                .clicked()
            {
                self.collapsed_folders = file_tree::rows(data, "", &Default::default())
                    .iter()
                    .filter(|r| r["__folder"].as_bool() == Some(true))
                    .filter_map(|r| r["__path"].as_str().map(str::to_owned))
                    .collect();
            }
        });
        let rows = file_tree::rows(data, &self.file_filter, &self.collapsed_folders);
        let searching = !self.file_filter.trim().is_empty();
        let (toggled, rename) = file_tree_table(ui, hash, &rows, searching, |ui, row| {
            if row["__folder"].as_bool() == Some(true) {
                return;
            }
            let mut priority = number(row, "priority");
            let old = priority;
            egui::ComboBox::from_id_salt(("prio", number(row, "index")))
                .selected_text(priority_name(priority))
                .show_ui(ui, |ui| {
                    for p in [0, 1, 6, 7] {
                        ui.selectable_value(&mut priority, p, priority_name(p));
                    }
                });
            if old != priority {
                self.action(
                    "Приоритет файла",
                    "torrents/filePrio",
                    vec![
                        ("hash".into(), hash.into()),
                        ("id".into(), number(row, "index").to_string()),
                        ("priority".into(), priority.to_string()),
                    ],
                );
            }
            if ui.small_button("Имя…").clicked() {
                self.open_form(
                    "Переименовать файл",
                    "torrents/renameFile",
                    vec![
                        ("hash".into(), hash.into()),
                        ("oldPath".into(), text(row, "__path")),
                    ],
                    vec![Field::new(
                        "newPath",
                        "Новое имя / относительный путь",
                        text(row, "__path"),
                    )],
                    "Файл будет переименован на сервере.",
                );
            }
        });
        if let Some(path) = toggled {
            if !self.collapsed_folders.remove(&path) {
                self.collapsed_folders.insert(path);
            }
        }
        if let Some(path) = rename {
            self.open_form(
                "Переименовать папку",
                "torrents/renameFolder",
                vec![
                    ("hash".into(), hash.into()),
                    ("oldPath".into(), path.clone()),
                ],
                vec![Field::new("newPath", "Новый относительный путь", path)],
                "Папка будет переименована на сервере.",
            );
        }
    }
}

fn file_tree_table(
    ui: &mut egui::Ui,
    hash: &str,
    rows: &[Value],
    searching: bool,
    mut action: impl FnMut(&mut egui::Ui, &Value),
) -> (Option<String>, Option<String>) {
    let mut toggled = None;
    let mut rename = None;
    if rows.is_empty() {
        ui.label(if searching {
            "Файлы не найдены"
        } else {
            "Нет файлов · метаданные ещё не получены"
        });
        return (None, None);
    }
    let width = ui.available_width().max(850.0);
    egui::ScrollArea::horizontal()
        .id_salt(("file-tree-scroll", hash))
        .show(ui, |ui| {
            ui.set_width(width);
            TableBuilder::new(ui)
                .id_salt(("file-tree", hash))
                .striped(true)
                .sense(egui::Sense::click())
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::remainder().at_least(280.0).clip(true))
                .column(Column::initial(100.0))
                .column(Column::initial(95.0))
                .column(Column::initial(100.0))
                .column(Column::initial(220.0))
                .header(22.0, |mut header| {
                    for title in ["Имя", "Размер", "Прогресс", "Доступность", "Действия"]
                    {
                        header.col(|ui| {
                            ui.weak(title);
                        });
                    }
                })
                .body(|body| {
                    body.rows(20.0, rows.len(), |mut row| {
                        let file = &rows[row.index()];
                        let folder = file["__folder"].as_bool() == Some(true);
                        row.col(|ui| {
                            ui.add_space(number(file, "__depth") as f32 * 16.0);
                            if folder {
                                let arrow = if file["__open"].as_bool() == Some(true) {
                                    "▼"
                                } else {
                                    "▶"
                                };
                                if ui
                                    .add(
                                        egui::Label::new(
                                            RichText::new(arrow)
                                                .color(crate::theme::palette(ui.ctx()).orange),
                                        )
                                        .selectable(false)
                                        .sense(egui::Sense::click()),
                                    )
                                    .clicked()
                                    && !searching
                                {
                                    toggled = Some(text(file, "__path"));
                                }
                                let label = format!(
                                    "{}  ({})",
                                    text(file, "name"),
                                    number(file, "__count")
                                );
                                let response = ui
                                    .add(
                                        egui::Label::new(
                                            RichText::new(label)
                                                .color(crate::theme::palette(ui.ctx()).orange),
                                        )
                                        .truncate()
                                        .selectable(false)
                                        .sense(egui::Sense::click()),
                                    )
                                    .on_hover_text(if searching {
                                        "Папки раскрыты для результатов поиска".into()
                                    } else {
                                        text(file, "__path")
                                    });
                                if response.double_clicked() {
                                    rename = Some(text(file, "__path"));
                                }
                                response.context_menu(|ui| {
                                    if action_menu_item(ui, "Переименовать папку…", 10, false)
                                        .clicked()
                                    {
                                        rename = Some(text(file, "__path"));
                                        ui.close_menu();
                                    }
                                });
                            } else {
                                ui.add_space(18.0);
                                ui.add(egui::Label::new(text(file, "name")).truncate())
                                    .on_hover_text(text(file, "__path"));
                            }
                        });
                        row.col(|ui| {
                            ui.label(format_value("size", &file["size"]));
                        });
                        row.col(|ui| {
                            ui.label(format_value("progress", &file["progress"]));
                        });
                        row.col(|ui| {
                            ui.label(format_value("availability", &file["availability"]));
                        });
                        row.col(|ui| {
                            ui.horizontal(|ui| action(ui, file));
                        });
                        if folder {
                            row.response().context_menu(|ui| {
                                if action_menu_item(ui, "Переименовать папку…", 10, false).clicked()
                                {
                                    rename = Some(text(file, "__path"));
                                    ui.close_menu();
                                }
                            });
                        }
                    });
                });
        });
    (toggled, rename)
}

fn peer_address(ip: &str, port: i64) -> String {
    if ip.contains(':') && !ip.starts_with('[') {
        format!("[{ip}]:{port}")
    } else {
        format!("{ip}:{port}")
    }
}
fn push_history(history: &mut VecDeque<(f32, f32)>, down: f32, up: f32) {
    history.push_back((down, up));
    if history.len() > 120 {
        history.pop_front();
    }
}
pub fn kib_to_bytes(input: &str) -> Result<i64, String> {
    let n: f64 = input
        .trim()
        .replace(',', ".")
        .parse()
        .map_err(|_| "Введите число в КиБ/с")?;
    if !n.is_finite() || n < 0.0 || n * 1024.0 > i32::MAX as f64 {
        return Err("Лимит должен быть от 0 до 2097151 КиБ/с".into());
    }
    Ok((n * 1024.0).round() as i64)
}
fn number(v: &Value, key: &str) -> i64 {
    v[key].as_i64().unwrap_or(0)
}
fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or("").to_string()
}
fn priority_name(p: i64) -> &'static str {
    match p {
        0 => "Не скачивать",
        6 => "Высокий",
        7 => "Максимальный",
        _ => "Обычный",
    }
}
fn speed_graph(ui: &mut egui::Ui, history: &VecDeque<(f32, f32)>) {
    ui.label("История текущего запуска · скачивание / отдача · обновление раз в 5 секунд");
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 150.0),
        egui::Sense::hover(),
    );
    let peak = history
        .iter()
        .flat_map(|(a, b)| [*a, *b])
        .fold(1.0_f32, f32::max);
    ui.painter()
        .rect_filled(rect, 4.0, crate::theme::palette(ui.ctx()).input);
    for (idx, color) in [
        (0, crate::theme::palette(ui.ctx()).blue),
        (1, crate::theme::palette(ui.ctx()).green),
    ] {
        let points: Vec<_> = history
            .iter()
            .enumerate()
            .map(|(i, (a, b))| {
                egui::pos2(
                    rect.left() + i as f32 * rect.width() / 119.0,
                    rect.bottom() - (if idx == 0 { *a } else { *b }) / peak * rect.height(),
                )
            })
            .collect();
        if points.len() > 1 {
            ui.painter()
                .add(egui::Shape::line(points, egui::Stroke::new(1.5_f32, color)));
        }
    }
    ui.label(format!("Верхняя граница: {}/с", bytes(peak as u64)));
}
fn data_table(
    ui: &mut egui::Ui,
    id: &str,
    rows: &[Value],
    columns: &[(&str, &str)],
    mut action: impl FnMut(&mut egui::Ui, &Value),
) {
    if rows.is_empty() {
        ui.label("Нет данных");
        return;
    }
    let width = ui
        .available_width()
        .max(columns.len() as f32 * 130.0 + 230.0);
    egui::ScrollArea::horizontal().id_salt(id).show(ui, |ui| {
        ui.set_width(width);
        let mut table = TableBuilder::new(ui)
            .id_salt(id)
            .resizable(true)
            .column(Column::remainder().at_least(200.0));
        for _ in 1..columns.len() {
            table = table.column(Column::initial(110.0));
        }
        table
            .column(Column::initial(210.0))
            .header(22.0, |mut h| {
                for (_, label) in columns {
                    h.col(|ui| {
                        ui.weak(*label);
                    });
                }
                h.col(|ui| {
                    ui.weak("Действия");
                });
            })
            .body(|body| {
                body.rows(20.0, rows.len(), |mut r| {
                    let item = &rows[r.index()];
                    for (key, _) in columns {
                        r.col(|ui| {
                            ui.add(egui::Label::new(format_value(key, &item[*key])).truncate())
                                .on_hover_text(format_value(key, &item[*key]));
                        });
                    }
                    r.col(|ui| {
                        ui.horizontal(|ui| action(ui, item));
                    });
                });
            });
    });
}
fn object_grid(ui: &mut egui::Ui, id: &str, value: &Value) {
    if let Some(map) = value.as_object() {
        egui::Grid::new(id)
            .num_columns(2)
            .striped(true)
            .spacing([28.0, 3.5])
            .show(ui, |ui| {
                for (key, value) in map {
                    ui.weak(label(key));
                    ui.add(egui::Label::new(format_value(key, value)).wrap())
                        .context_menu(|ui| {
                            if ui.button("Копировать").clicked() {
                                ui.ctx().copy_text(format_value(key, value));
                                ui.close_menu();
                            }
                        });
                    ui.end_row();
                }
            });
    }
}
fn format_value(key: &str, value: &Value) -> String {
    if value.is_null() {
        return "—".into();
    }
    if let Some(b) = value.as_bool() {
        return if b { "Да" } else { "Нет" }.into();
    }
    if let Some(s) = value.as_str() {
        return if s.is_empty() { "—".into() } else { s.into() };
    }
    if key == "progress" {
        return format!("{:.1}%", value.as_f64().unwrap_or(0.0) * 100.0);
    }
    if matches!(
        key,
        "addition_date"
            | "completion_date"
            | "creation_date"
            | "last_seen"
            | "added_on"
            | "completion_on"
            | "last_activity"
    ) {
        return chrono::DateTime::from_timestamp(value.as_i64().unwrap_or(0), 0)
            .filter(|d| d.timestamp() > 0)
            .map(|d| {
                d.with_timezone(&chrono::Local)
                    .format("%d.%m.%Y %H:%M")
                    .to_string()
            })
            .unwrap_or_else(|| "—".into());
    }
    if matches!(key, "eta" | "time_elapsed" | "seeding_time" | "reannounce") {
        let n = value.as_i64().unwrap_or(0);
        return if !(0..8640000).contains(&n) {
            "—".into()
        } else {
            format!("{} д {} ч {} мин", n / 86400, n / 3600 % 24, n / 60 % 60)
        };
    }
    if key.ends_with("limit") && value.as_i64().is_some_and(|n| n <= 0) {
        return "Без ограничения".into();
    }
    if key.contains("speed")
        || matches!(
            key,
            "dl_limit" | "up_limit" | "dl_rate_limit" | "up_rate_limit"
        )
    {
        let n = value.as_i64().unwrap_or(0);
        return if n < 0 {
            "—".into()
        } else {
            format!("{}/с", bytes(n as u64))
        };
    }
    if matches!(
        key,
        "size"
            | "total_size"
            | "piece_size"
            | "total_downloaded"
            | "total_uploaded"
            | "total_wasted"
            | "total_downloaded_session"
            | "total_uploaded_session"
            | "downloaded"
            | "uploaded"
            | "dl_info_data"
            | "up_info_data"
            | "free_space_on_disk"
    ) {
        return value.as_u64().map(bytes).unwrap_or_else(|| "—".into());
    }
    value.to_string()
}
fn label(key: &str) -> &str {
    match key {
        "name" => "Название",
        "save_path" => "Путь сохранения",
        "download_path" => "Временная папка",
        "comment" => "Комментарий",
        "created_by" => "Создан программой",
        "creation_date" => "Дата создания",
        "addition_date" => "Добавлен",
        "completion_date" => "Завершён",
        "last_seen" => "Последний полный источник",
        "time_elapsed" => "Активное время",
        "seeding_time" => "Время раздачи",
        "eta" => "Осталось",
        "total_downloaded" => "Всего скачано",
        "total_uploaded" => "Всего отдано",
        "total_wasted" => "Потеряно",
        "total_downloaded_session" => "Скачано за сеанс",
        "total_uploaded_session" => "Отдано за сеанс",
        "dl_speed" => "Скачивание",
        "up_speed" => "Отдача",
        "dl_speed_avg" => "Средняя скорость скачивания",
        "up_speed_avg" => "Средняя скорость отдачи",
        "dl_limit" => "Лимит скачивания",
        "up_limit" => "Лимит отдачи",
        "nb_connections" => "Соединения",
        "nb_connections_limit" => "Лимит соединений",
        "seeds" => "Подключённые сиды",
        "seeds_total" => "Всего сидов",
        "peers" => "Подключённые личи",
        "peers_total" => "Всего личей",
        "share_ratio" => "Рейтинг",
        "popularity" => "Популярность",
        "reannounce" => "Следующий анонс",
        "total_size" => "Полный размер",
        "pieces_num" => "Число частей",
        "pieces_have" => "Готовые части",
        "piece_size" => "Размер части",
        "private" | "is_private" => "Приватный",
        "has_metadata" => "Метаданные получены",
        "progress" => "Прогресс",
        "infohash_v1" => "Инфохеш v1",
        "infohash_v2" => "Инфохеш v2",
        "availability" => "Доступность",
        "dl_info_speed" => "Скорость скачивания",
        "up_info_speed" => "Скорость отдачи",
        "dl_info_data" => "Скачано за сеанс",
        "up_info_data" => "Отдано за сеанс",
        "dl_rate_limit" => "Лимит скачивания",
        "up_rate_limit" => "Лимит отдачи",
        "dht_nodes" => "Узлы DHT",
        "connection_status" => "Статус соединения",
        _ => key,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn folder_double_click_and_context_menu_target_the_folder() {
        use super::*;
        for context_menu in [false, true] {
            let ctx = egui::Context::default();
            let rows = file_tree::rows(
                &json!([{"name":"Folder/file.txt", "index":0, "size":10}]),
                "",
                &Default::default(),
            );
            let mut target = egui::Pos2::ZERO;
            let mut renamed = None;
            for frame in 0..6 {
                let mut input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 700.0),
                    )),
                    time: Some(frame as f64 * 0.08),
                    ..Default::default()
                };
                if frame > 0 && !(context_menu && frame == 3) {
                    input.events = vec![
                        egui::Event::PointerMoved(target),
                        egui::Event::PointerButton {
                            pos: target,
                            button: if context_menu && frame <= 2 {
                                egui::PointerButton::Secondary
                            } else {
                                egui::PointerButton::Primary
                            },
                            pressed: if context_menu && frame >= 4 {
                                frame == 4
                            } else {
                                frame % 2 == 1
                            },
                            modifiers: egui::Modifiers::NONE,
                        },
                    ];
                }
                let output = ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let (toggle, rename) =
                            file_tree_table(ui, "test-hash", &rows, false, |_, _| {});
                        assert!(toggle.is_none(), "Renaming must not collapse the folder");
                        if rename.is_some() {
                            renamed = rename;
                        }
                    });
                });
                if frame == 0 || (context_menu && frame == 3) {
                    let label = if frame == 0 {
                        "Folder  (1)"
                    } else {
                        "Переименовать папку…"
                    };
                    target = output
                        .shapes
                        .iter()
                        .find_map(|shape| {
                            if let egui::Shape::Text(text) = &shape.shape {
                                if text.galley.text() == label {
                                    return Some(text.pos + egui::vec2(4.0, 4.0));
                                }
                            }
                            None
                        })
                        .unwrap_or_else(|| {
                            panic!("Missing {label}, context={context_menu}, frame={frame}")
                        });
                }
            }
            assert_eq!(renamed.as_deref(), Some("Folder"));
        }
    }
    use super::*;
    #[test]
    fn ipv6_peer_uses_brackets() {
        assert_eq!(peer_address("2001:db8::1", 6881), "[2001:db8::1]:6881");
        assert_eq!(peer_address("127.0.0.1", 80), "127.0.0.1:80");
    }
    #[test]
    fn stale_session_reply_is_ignored_and_requests_are_deduplicated() {
        let mut remote = Remote::default();
        remote.get("transfer", "transfer/info", vec![]);
        remote.get("transfer", "transfer/info", vec![]);
        assert_eq!(remote.commands().len(), 1);
        let old = remote.serial;
        remote.clear();
        remote.get("transfer", "transfer/info", vec![]);
        remote.reply(old, Ok(json!({"dl_info_speed":99})));
        assert!(!remote.cache.contains_key("transfer"));
        remote.reply(remote.serial, Ok(json!({"dl_info_speed":10})));
        assert_eq!(
            remote.get("transfer", "transfer/info", vec![]).unwrap()["dl_info_speed"],
            10
        );
    }
    #[test]
    fn failed_preference_save_keeps_edits() {
        let mut remote = Remote {
            prefs: Some(json!({"up_limit":1024})),
            ..Default::default()
        };
        remote.action("Сохранить настройки", "app/setPreferences", vec![]);
        remote.reply(remote.serial, Err("HTTP 403".into()));
        assert_eq!(remote.prefs.unwrap()["up_limit"], 1024);
        assert!(remote.error);
    }
    #[test]
    fn speed_units_are_checked() {
        assert_eq!(kib_to_bytes("1.5").unwrap(), 1536);
        assert_eq!(kib_to_bytes("0").unwrap(), 0);
        for bad in ["-1", "NaN", "inf", "999999999999"] {
            assert!(kib_to_bytes(bad).is_err());
        }
    }
}
