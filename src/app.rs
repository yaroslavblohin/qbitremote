use crate::{
    api::{self, Command, Event},
    model::{bytes, Settings, Torrent},
};
use eframe::egui::{self, Color32, RichText};
use egui_extras::{Column, TableBuilder};
use std::sync::mpsc::{Receiver, Sender};

const MUTED: Color32 = Color32::from_rgb(146, 163, 184);
const ACCENT: Color32 = Color32::from_rgb(64, 199, 191);
const BLUE: Color32 = Color32::from_rgb(84, 169, 255);
const GREEN: Color32 = Color32::from_rgb(87, 211, 150);
const ORANGE: Color32 = Color32::from_rgb(255, 181, 93);
const RED: Color32 = Color32::from_rgb(249, 113, 132);
const SURFACE: Color32 = Color32::from_rgb(24, 33, 46);

pub struct ClientApp {
    about_open: bool,
    dragged_column: Option<String>,
    column_layout: Vec<String>,
    remote: crate::remote::Remote,
    settings: Settings,
    password: String,
    tx: Sender<Command>,
    rx: Receiver<(u64, Event)>,
    connection_epoch: u64,
    torrents: Vec<Torrent>,
    selected: Option<String>,
    filter: usize,
    search: String,
    connected: bool,
    connecting: bool,
    version: String,
    notice: String,
    is_error: bool,
    settings_open: bool,
    add_open: bool,
    links: String,
    save_path: String,
    delete: Option<(String, String)>,
    delete_files: bool,
    sort: usize,
    descending: bool,
    #[cfg(windows)]
    tray: Option<crate::tray::Tray>,
}
impl ClientApp {
    #[cfg(feature = "ui-smoke")]
    pub fn smoke_tray_control(&mut self) -> crate::tray::WindowControl {
        self.settings.close_to_tray = true;
        self.tray.as_ref().expect("tray required").control.clone()
    }
    #[cfg(feature = "ui-smoke")]
    pub fn smoke_fixture(&mut self) {
        if std::env::var_os("QBIT_SMOKE_COLUMNS").is_some() {
            self.settings.visible_columns = crate::columns::COLUMNS
                .iter()
                .map(|c| c.key.to_owned())
                .collect();
        }
        self.remote.smoke_fixture();
        self.settings.close_to_tray = false;
        self.settings.zoom = std::env::var("QBIT_SMOKE_ZOOM")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.0);
        self.connected = true;
        self.version = "5.x · TEST".into();
        self.notice = "Тестовые данные · проверка интерфейса без подключения к серверу".into();
        self.torrents = vec![
            Torrent {
                hash: "fixture-1".into(),
                name: "Ubuntu 24.04 LTS — Desktop amd64.iso".into(),
                size: 6_000_000_000,
                progress: 0.64,
                dlspeed: 12_450_000,
                state: "downloading".into(),
                category: "Linux".into(),
                save_path: "D:/Downloads/Linux".into(),
                num_seeds: 24,
                num_leechs: 8,
                eta: 240,
                ..Default::default()
            },
            Torrent {
                hash: "fixture-2".into(),
                name: "Blender — открытые материалы".into(),
                size: 1_200_000_000,
                progress: 1.0,
                upspeed: 2_100_000,
                state: "uploading".into(),
                ..Default::default()
            },
            Torrent {
                hash: "fixture-3".into(),
                name: "Debian — образ установочного диска.iso".into(),
                size: 4_800_000_000,
                progress: 0.28,
                state: "stoppedDL".into(),
                ..Default::default()
            },
        ];
        self.selected = Some("fixture-1".into());
        #[cfg(windows)]
        assert!(self.tray.is_some(), "Windows tray creation failed");
    }
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = &cc.egui_ctx;
        ctx.set_theme(egui::Theme::Dark);
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = SURFACE;
        visuals.window_fill = Color32::from_rgb(30, 41, 57);
        visuals.extreme_bg_color = Color32::from_rgb(16, 24, 35);
        visuals.faint_bg_color = Color32::from_rgb(28, 39, 54);
        visuals.override_text_color = Some(Color32::from_rgb(226, 228, 231));
        visuals.widgets.inactive.bg_fill = Color32::from_rgb(36, 49, 66);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(44, 65, 84);
        visuals.widgets.active.bg_fill = Color32::from_rgb(37, 84, 96);
        visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0_f32, Color32::from_rgb(44, 59, 77));
        visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
        visuals.hyperlink_color = BLUE;
        visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(6);
        visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(6);
        visuals.widgets.active.corner_radius = egui::CornerRadius::same(6);
        visuals.selection.bg_fill = Color32::from_rgb(32, 68, 83);
        visuals.selection.stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(230, 242, 249));
        ctx.set_visuals(visuals);
        ctx.style_mut(|s| {
            s.spacing.item_spacing = egui::vec2(8.0, 4.0);
            s.spacing.button_padding = egui::vec2(10.0, 3.0);
            s.spacing.interact_size.y = 18.0;
        });
        // Use the installed Windows font for Cyrillic and crisp scalable text.
        if let Ok(font) = std::fs::read("C:/Windows/Fonts/segoeui.ttf") {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("segoe".into(), egui::FontData::from_owned(font).into());
            fonts
                .families
                .get_mut(&egui::FontFamily::Proportional)
                .unwrap()
                .insert(0, "segoe".into());
            ctx.set_fonts(fonts);
        }
        let stored: Option<Settings> = cc.storage.and_then(|s| eframe::get_value(s, "settings"));
        let has_saved_settings = stored.is_some();
        let mut settings = stored.unwrap_or_default();
        if !settings.profiles_initialized && has_saved_settings {
            settings.save_server();
        }
        settings.profiles_initialized = true;
        crate::columns::normalize(&mut settings.visible_columns);
        if !settings.servers.is_empty() {
            settings.select_server(settings.selected_server.min(settings.servers.len() - 1));
        }
        settings.zoom = settings.zoom.clamp(0.8, 1.5);
        ctx.set_zoom_factor(settings.zoom);
        let (tx, rx) = api::worker(ctx.clone());
        #[cfg(windows)]
        let (tray, tray_error) = match crate::tray::Tray::new(cc) {
            Ok(tray) => (Some(tray), None),
            Err(e) => (None, Some(format!("Системный трей недоступен: {e}"))),
        };
        let password_result = crate::credentials::load(&settings);
        let password_error = password_result.as_ref().err().cloned();
        let auto_connect = !settings.servers.is_empty()
            && settings.auto_connect
            && password_error.is_none()
            && !cfg!(feature = "ui-smoke");
        let (sort, descending) = crate::columns::restored_sort(&settings);
        let mut app = Self {
            about_open: false,
            dragged_column: None,
            column_layout: Vec::new(),
            remote: crate::remote::Remote::default(),
            settings,
            password: password_result.unwrap_or_default(),
            tx,
            rx,
            connection_epoch: 0,
            torrents: vec![],
            selected: None,
            filter: 0,
            search: String::new(),
            connected: false,
            connecting: false,
            version: String::new(),
            notice: String::new(),
            is_error: false,
            settings_open: false,
            add_open: false,
            links: String::new(),
            save_path: String::new(),
            delete: None,
            delete_files: false,
            sort,
            descending,
            #[cfg(windows)]
            tray,
        }
        .with_tray_notice(
            #[cfg(windows)]
            tray_error.or(password_error),
            #[cfg(not(windows))]
            None,
        );
        if auto_connect {
            app.connect();
        }
        app
    }
    fn with_tray_notice(mut self, error: Option<String>) -> Self {
        if let Some(error) = error {
            self.notice = error;
            self.is_error = true;
        }
        self
    }
    fn send(&mut self, command: Command) {
        if self.tx.send(command).is_err() {
            self.notice = "Сетевой поток завершился. Перезапустите приложение.".into();
            self.is_error = true;
        }
    }
    fn connect(&mut self) {
        self.settings.save_server();
        self.connection_epoch += 1;
        self.remote.clear();
        self.delete = None;
        self.connecting = true;
        self.connected = false;
        self.torrents.clear();
        self.selected = None;
        self.add_open = false;
        self.links.clear();
        self.save_path.clear();
        self.version.clear();
        self.notice = "Подключение…".into();
        self.is_error = false;
        let password = self.password.clone();
        self.send(Command::Connect(
            Box::new(self.settings.clone()),
            password,
            self.connection_epoch,
        ));
    }
    fn disconnect(&mut self) {
        self.connection_epoch += 1;
        self.send(Command::Disconnect(self.connection_epoch));
        self.connected = false;
        self.connecting = false;
        self.remote.clear();
        self.torrents.clear();
        self.selected = None;
        self.delete = None;
        self.add_open = false;
        self.notice = "Отключено".into();
    }

    fn select_server(&mut self, index: usize) {
        self.settings.save_server();
        self.settings.select_server(index);
        match crate::credentials::load(&self.settings) {
            Ok(password) => {
                self.password = password;
                self.connect();
            }
            Err(error) => {
                self.disconnect();
                self.password.clear();
                self.notice = error;
                self.is_error = true;
                self.settings_open = true;
            }
        }
    }
    fn receive(&mut self) {
        while let Ok((epoch, event)) = self.rx.try_recv() {
            if epoch != self.connection_epoch {
                continue;
            }
            match event {
                Event::Connected(v) => {
                    self.version = v;
                    self.connected = true;
                    self.connecting = false;
                    self.settings_open = false;
                    self.notice.clear();
                    self.is_error = false;
                    if let Err(error) =
                        crate::credentials::store(&mut self.settings, &self.password)
                    {
                        self.notice = format!("Подключено. Пароль не сохранён: {error}");
                        self.is_error = true;
                    }
                    self.settings.save_server();
                }
                Event::Remote { token, result } => self.remote.reply(token, result),
                Event::Disconnected => {
                    self.remote.clear();
                    self.delete = None;
                    self.connected = false;
                    self.connecting = false;
                    self.torrents.clear();
                    self.selected = None;
                }
                Event::Torrents(torrents) => {
                    self.torrents = torrents;
                    if !self
                        .torrents
                        .iter()
                        .any(|t| Some(&t.hash) == self.selected.as_ref())
                    {
                        self.selected = None;
                    }
                }
                Event::Error(error) => {
                    self.notice = error;
                    self.is_error = true;
                }
                Event::Done(message) => {
                    self.notice = message;
                    self.is_error = false;
                }
            }
        }
    }
    fn matches_filter(t: &Torrent, filter: usize) -> bool {
        match filter {
            1 => t.downloading(),
            2 => t.progress >= 1.0 && !t.stopped(),
            3 => t.stopped(),
            4 => matches!(t.state.as_str(), "error" | "missingFiles"),
            _ => true,
        }
    }
    fn server_picker(&mut self, ui: &mut egui::Ui, id: &str) {
        let mut selected = self.settings.selected_server;
        egui::ComboBox::from_id_salt(id)
            .selected_text(&self.settings.server_name)
            .width(160.0)
            .show_ui(ui, |ui| {
                for (index, server) in self.settings.servers.iter().enumerate() {
                    ui.selectable_value(&mut selected, index, &server.name)
                        .on_hover_text(&server.address);
                }
            });
        if selected != self.settings.selected_server {
            self.select_server(selected);
        }
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        egui::Window::new("О программе")
            .open(&mut self.about_open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("qBit Remote");
                ui.label(format!("Версия {}", env!("CARGO_PKG_VERSION")));
                ui.label("Клиент для удалённого управления qBittorrent.");
                ui.separator();
                ui.label("Щелчок по заголовку — сортировка.");
                ui.label("Перетаскивание заголовка — порядок колонок.");
                ui.label("Перетаскивание границы — ширина колонки.");
                ui.label("Вид → Колонки — выбор отображаемых полей.");
            });
        let mut open = self.settings_open;
        egui::Window::new("Подключение и настройки")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(560.0)
            .vscroll(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    self.server_picker(ui, "settings-servers");
                    if ui.button("+ Сервер").clicked() {
                        self.settings.save_server();
                        self.disconnect();
                        self.settings.selected_server = self.settings.servers.len();
                        self.settings.server_name =
                            format!("Сервер {}", self.settings.servers.len() + 1);
                        self.settings.address = "http://localhost:8080".into();
                        self.settings.username = "admin".into();
                        self.settings.remember_password = true;
                        self.settings.saved_password = None;
                        self.password.clear();
                        self.settings.save_server();
                    }
                    if ui
                        .add_enabled(
                            !self.connecting && !self.settings.servers.is_empty(),
                            egui::Button::new("Удалить профиль"),
                        )
                        .clicked()
                    {
                        self.disconnect();
                        self.settings.servers.remove(self.settings.selected_server);
                        if self.settings.servers.is_empty() {
                            self.settings.selected_server = 0;
                            self.settings.server_name = "Основной сервер".into();
                            self.settings.address = "http://localhost:8080".into();
                            self.settings.username = "admin".into();
                            self.settings.saved_password = None;
                            self.password.clear();
                        } else {
                            self.settings.select_server(0);
                            self.password =
                                crate::credentials::load(&self.settings).unwrap_or_default();
                        }
                    }
                });
                ui.label("Удалённый сервер qBittorrent");
                ui.label(RichText::new("Включите Web UI в настройках сервера.").color(MUTED));
                ui.add_enabled_ui(!self.connecting && !self.connected, |ui| {
                    ui.label("Название сервера");
                    ui.text_edit_singleline(&mut self.settings.server_name);
                    ui.label("Адрес Web UI");
                    let address_changed = ui
                        .add(
                            egui::TextEdit::singleline(&mut self.settings.address)
                                .desired_width(f32::INFINITY),
                        )
                        .changed();
                    ui.label("Имя пользователя");
                    let username_changed = ui
                        .text_edit_singleline(&mut self.settings.username)
                        .changed();
                    if address_changed || username_changed {
                        self.password =
                            crate::credentials::load(&self.settings).unwrap_or_default();
                        self.settings.saved_password = None;
                    }
                    ui.label("Пароль");
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                });
                if ui
                    .add_enabled(
                        !self.connecting,
                        egui::Checkbox::new(
                            &mut self.settings.remember_password,
                            "Сохранять пароль на этом компьютере",
                        ),
                    )
                    .changed()
                {
                    if !self.settings.remember_password {
                        self.settings.saved_password = None;
                    }
                    if self.connected && self.settings.remember_password {
                        if let Err(error) =
                            crate::credentials::store(&mut self.settings, &self.password)
                        {
                            self.notice = error;
                            self.is_error = true;
                        }
                    }
                }
                ui.small("Пароль защищён учётной записью Windows.");
                ui.checkbox(
                    &mut self.settings.auto_connect,
                    "Подключаться к выбранному серверу при запуске",
                );
                if ui
                    .add_enabled(!self.connecting, egui::Button::new("Сохранить профиль"))
                    .clicked()
                {
                    match crate::credentials::store(&mut self.settings, &self.password) {
                        Ok(()) => {
                            self.notice = "Профиль сохранён".into();
                            self.is_error = false;
                        }
                        Err(error) => {
                            self.notice = error;
                            self.is_error = true;
                        }
                    }
                    self.settings.save_server();
                }
                if self.connected {
                    if ui.button("Отключиться").clicked() {
                        self.disconnect();
                    }
                } else if ui
                    .add_enabled(
                        !self.connecting,
                        egui::Button::new(if self.connecting {
                            "Подключение…"
                        } else {
                            "Подключиться"
                        }),
                    )
                    .clicked()
                {
                    self.connect();
                }
                ui.separator();
                ui.checkbox(
                    &mut self.settings.close_to_tray,
                    "Закрытие окна сворачивает в трей",
                );
                ui.label("Масштаб интерфейса · поверх системного DPI");
                if ui
                    .add(egui::Slider::new(&mut self.settings.zoom, 0.8..=1.5).suffix("×"))
                    .changed()
                {
                    ctx.set_zoom_factor(self.settings.zoom);
                }
                ui.label(
                    RichText::new("Выход — через меню «Файл» или значок в трее.").color(MUTED),
                );
            });
        self.settings_open = open && self.settings_open;
        let mut add_open = self.add_open;
        egui::Window::new("Добавить торрент")
            .open(&mut add_open)
            .collapsible(false)
            .default_width(480.0)
            .show(ctx, |ui| {
                ui.label("Magnet-ссылки или URL · по одной на строку");
                ui.add(
                    egui::TextEdit::multiline(&mut self.links)
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                );
                ui.label("Папка на сервере (необязательно)");
                ui.add(
                    egui::TextEdit::singleline(&mut self.save_path).desired_width(f32::INFINITY),
                );
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.links.trim().is_empty() && self.connected,
                            egui::Button::new("Добавить ссылки"),
                        )
                        .clicked()
                    {
                        self.send(Command::Add(
                            self.links.trim().into(),
                            self.save_path.clone(),
                        ));
                        self.links.clear();
                        self.add_open = false;
                    }
                    if ui
                        .add_enabled(self.connected, egui::Button::new("Выбрать .torrent…"))
                        .clicked()
                    {
                        if let Some(file) = rfd::FileDialog::new()
                            .add_filter("Torrent", &["torrent"])
                            .pick_file()
                        {
                            self.send(Command::File(file, self.save_path.clone()));
                            self.add_open = false;
                        }
                    }
                });
            });
        self.add_open = add_open && self.add_open;
        if let Some((hash, name)) = self.delete.clone() {
            egui::Window::new("Удалить торрент?")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(name);
                    ui.checkbox(&mut self.delete_files, "Также удалить файлы на сервере");
                    if self.delete_files {
                        ui.colored_label(
                            Color32::LIGHT_RED,
                            "Файлы будут удалены с удалённого сервера.",
                        );
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Отмена").clicked() {
                            self.delete = None;
                        }
                        if ui
                            .add_enabled(self.connected, egui::Button::new("Удалить"))
                            .clicked()
                        {
                            self.send(Command::Delete(hash, self.delete_files));
                            self.delete = None;
                        }
                    });
                });
        }
    }
    fn column_picker(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Колонки", |ui| {
            ui.horizontal(|ui| {
                if ui.button("Все").clicked() {
                    self.settings.visible_columns = crate::columns::COLUMNS
                        .iter()
                        .map(|c| c.key.to_owned())
                        .collect();
                }
                if ui.button("По умолчанию").clicked() {
                    self.settings.visible_columns = crate::columns::defaults();
                    self.sort = 0;
                    self.descending = false;
                }
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height(380.0)
                .show(ui, |ui| {
                    for (index, column) in crate::columns::COLUMNS.iter().enumerate() {
                        let mut visible = self
                            .settings
                            .visible_columns
                            .iter()
                            .any(|key| key == column.key);
                        let enabled = !visible || self.settings.visible_columns.len() > 1;
                        if ui
                            .add_enabled(enabled, egui::Checkbox::new(&mut visible, column.title))
                            .changed()
                        {
                            if visible {
                                self.settings.visible_columns.push(column.key.into());
                            } else {
                                self.settings
                                    .visible_columns
                                    .retain(|key| key != column.key);
                                if self.sort == index {
                                    self.sort = crate::columns::COLUMNS
                                        .iter()
                                        .position(|c| {
                                            self.settings
                                                .visible_columns
                                                .iter()
                                                .any(|key| key == c.key)
                                        })
                                        .unwrap_or(0);
                                }
                            }
                        }
                    }
                });
            ui.small("Отсутствующие на сервере данные: —");
        });
    }

    fn main_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("main-menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Файл", |ui| {
                    if ui
                        .add_enabled(self.connected, egui::Button::new("Добавить торрент…"))
                        .clicked()
                    {
                        self.add_open = true;
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(self.connected, egui::Button::new("Обновить список"))
                        .clicked()
                    {
                        self.send(Command::Refresh);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Выход").clicked() {
                        ui.close_menu();
                        #[cfg(windows)]
                        if let Some(tray) = &self.tray {
                            tray.control.exit();
                            return;
                        }
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Правка", |ui| {
                    let selected = self
                        .torrents
                        .iter()
                        .find(|t| Some(&t.hash) == self.selected.as_ref())
                        .cloned();
                    ui.add_enabled_ui(self.connected && selected.is_some(), |ui| {
                        if ui.button("Запустить торрент").clicked() {
                            self.send(Command::Start(selected.as_ref().unwrap().hash.clone()));
                            ui.close_menu();
                        }
                        if ui.button("Остановить торрент").clicked() {
                            self.send(Command::Stop(selected.as_ref().unwrap().hash.clone()));
                            ui.close_menu();
                        }
                        if ui.button("Удалить торрент…").clicked() {
                            let t = selected.as_ref().unwrap();
                            self.delete = Some((t.hash.clone(), t.name.clone()));
                            self.delete_files = false;
                            ui.close_menu();
                        }
                        if ui.button("Копировать название").clicked() {
                            ctx.copy_text(selected.as_ref().unwrap().name.clone());
                            ui.close_menu();
                        }
                    });
                    ui.separator();
                    if ui.button("Настройки приложения…").clicked() {
                        self.settings_open = true;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Вид", |ui| {
                    self.column_picker(ui);
                    ui.menu_button("Масштаб интерфейса", |ui| {
                        if ui
                            .add(egui::Slider::new(&mut self.settings.zoom, 0.8..=1.5).suffix("×"))
                            .changed()
                        {
                            ctx.set_zoom_factor(self.settings.zoom);
                        }
                        if ui.button("100%").clicked() {
                            self.settings.zoom = 1.0;
                            ctx.set_zoom_factor(1.0);
                        }
                    });
                    ui.separator();
                    for (page, title) in crate::remote::PAGES {
                        if page == crate::remote::Page::Preferences {
                            continue;
                        }
                        if ui
                            .selectable_label(self.remote.page == page, title)
                            .clicked()
                        {
                            self.remote.page = page;
                            ui.close_menu();
                        }
                    }
                });
                ui.menu_button("Серверы", |ui| {
                    let profiles: Vec<_> = self
                        .settings
                        .servers
                        .iter()
                        .map(|s| s.name.clone())
                        .collect();
                    for (index, name) in profiles.iter().enumerate() {
                        if ui
                            .selectable_label(self.settings.selected_server == index, name)
                            .clicked()
                        {
                            self.select_server(index);
                            ui.close_menu();
                        }
                    }
                    ui.separator();
                    if ui
                        .add_enabled(
                            !self.connecting && !self.connected,
                            egui::Button::new("Подключиться"),
                        )
                        .clicked()
                    {
                        self.connect();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(
                            self.connecting || self.connected,
                            egui::Button::new("Отключиться"),
                        )
                        .clicked()
                    {
                        self.disconnect();
                        ui.close_menu();
                    }
                    if ui.button("Управление серверами…").clicked() {
                        self.settings_open = true;
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(self.connected, egui::Button::new("Настройки qBittorrent…"))
                        .clicked()
                    {
                        self.remote.page = crate::remote::Page::Preferences;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Справка", |ui| {
                    if ui.button("О программе и управлении…").clicked() {
                        self.about_open = true;
                        ui.close_menu();
                    }
                });
            });
        });
    }
}
impl eframe::App for ClientApp {
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(3)
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.settings.sort_column = crate::columns::COLUMNS[self.sort].key.to_owned();
        self.settings.sort_descending = self.descending;
        if !self.settings.servers.is_empty() {
            self.settings.save_server();
        }
        eframe::set_value(storage, "settings", &self.settings);
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(feature = "ui-smoke")]
        ctx.set_zoom_factor(self.settings.zoom);
        self.receive();
        // Native restoration wakes hidden windows; mirror it into eframe's
        // cached viewport builder or the next Visible(false) is discarded.
        #[cfg(windows)]
        if self
            .tray
            .as_ref()
            .is_some_and(|t| t.control.take_restored())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.request_repaint();
        }
        #[cfg(windows)]
        if crate::tray::should_hide(
            ctx.input(|i| i.viewport().close_requested()),
            self.settings.close_to_tray,
            self.tray.is_some(),
            self.tray.as_ref().is_some_and(|t| t.control.quitting()),
        ) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
        self.main_menu(ctx);
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(
                    if self.connected { ACCENT } else { MUTED },
                    if self.connected {
                        "● Подключено"
                    } else if self.connecting {
                        "◌ Подключение"
                    } else {
                        "○ Нет подключения"
                    },
                );
                if self.connected {
                    ui.small(format!("qBittorrent {}", self.version));
                }
                ui.separator();
                ui.label(format!(
                    "↓ {}/с",
                    bytes(self.torrents.iter().map(|t| t.dlspeed).sum())
                ));
                ui.label(format!(
                    "↑ {}/с",
                    bytes(self.torrents.iter().map(|t| t.upspeed).sum())
                ));
            });
        });
        egui::SidePanel::left("sidebar")
            .exact_width(218.0)
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(18, 27, 39))
                    .inner_margin(14.0),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("navigation")
                    .show(ui, |ui| {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let (r, _) = ui
                                .allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::hover());
                            ui.painter().rect_filled(r, 9.0, ACCENT);
                            draw_icon(ui.painter(), r.center(), 1, Color32::from_rgb(13, 43, 48));
                            ui.label(RichText::new("qBit Remote").size(20.0).strong());
                        });
                        ui.add_space(12.0);
                        ui.label(RichText::new("Сервер").size(12.0).color(MUTED));
                        ui.add(
                            egui::Label::new(
                                RichText::new(&self.settings.server_name).color(ACCENT),
                            )
                            .truncate(),
                        )
                        .on_hover_text("Переключение и управление — в меню «Серверы»");
                        ui.add_space(24.0);
                        ui.label(RichText::new("Торренты").size(12.0).color(MUTED));
                        for (index, label) in [
                            "Все торренты",
                            "Загрузка",
                            "Раздача",
                            "Остановлены",
                            "Ошибки",
                        ]
                        .iter()
                        .enumerate()
                        {
                            let count = self
                                .torrents
                                .iter()
                                .filter(|t| Self::matches_filter(t, index))
                                .count();
                            if navigation_row(
                                ui,
                                label,
                                count,
                                index,
                                self.filter == index
                                    && self.remote.page == crate::remote::Page::Torrents,
                            )
                            .clicked()
                            {
                                self.filter = index;
                                self.remote.page = crate::remote::Page::Torrents;
                            }
                        }
                        ui.separator();
                        ui.label(RichText::new("ИНСТРУМЕНТЫ").size(10.0).color(MUTED));
                        for (index, (page, title)) in
                            crate::remote::PAGES.into_iter().skip(1).enumerate()
                        {
                            if page == crate::remote::Page::Preferences {
                                continue;
                            }
                            if navigation_row(
                                ui,
                                title,
                                usize::MAX,
                                index + 5,
                                self.remote.page == page,
                            )
                            .clicked()
                            {
                                self.remote.page = page;
                            }
                        }
                        ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                            ui.label(RichText::new("WINDOWS · 0.1.0").size(10.0).color(MUTED));
                            ui.add_space(8.0);
                        });
                    });
            });
        if self.remote.page == crate::remote::Page::Torrents {
            if let Some(t) = self
                .torrents
                .iter()
                .find(|t| Some(&t.hash) == self.selected.as_ref())
            {
                self.settings.details_height =
                    details_panel(ctx, self.settings.details_height, &t.name, |ui| {
                        self.remote.details(ui, t);
                    });
            }
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(SURFACE).inner_margin(16.0))
            .show(ctx, |ui| {
                if !self.remote.message.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(
                            if self.remote.error {
                                Color32::LIGHT_RED
                            } else {
                                MUTED
                            },
                            &self.remote.message,
                        );
                        if ui.small_button("×").clicked() {
                            self.remote.message.clear();
                        }
                    });
                }
                if self.connected && self.remote.page != crate::remote::Page::Torrents {
                    self.remote.page_ui(ui);
                    return;
                }
                ui.horizontal(|ui| {
                    ui.heading(
                        [
                            "Все торренты",
                            "Загрузка",
                            "Раздача",
                            "Остановлены",
                            "Ошибки",
                        ][self.filter],
                    );
                    ui.label(RichText::new(self.torrents.len().to_string()).color(MUTED));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add_enabled(
                                self.connected,
                                action_button("+  Добавить торрент", ACCENT),
                            )
                            .clicked()
                        {
                            self.add_open = true;
                        }
                    });
                });
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Поиск торрентов…")
                            .desired_width(180.0),
                    );
                    let selected = self.selected.clone();
                    ui.add_enabled_ui(self.connected && selected.is_some(), |ui| {
                        if ui.add(action_button("▶  Запустить", GREEN)).clicked() {
                            self.send(Command::Start(selected.clone().unwrap()));
                        }
                        if ui.add(action_button("II  Остановить", ORANGE)).clicked() {
                            self.send(Command::Stop(selected.clone().unwrap()));
                        }
                        if ui.add(action_button("×  Удалить", RED)).clicked() {
                            if let Some(t) = self
                                .torrents
                                .iter()
                                .find(|t| Some(&t.hash) == selected.as_ref())
                            {
                                self.delete = Some((t.hash.clone(), t.name.clone()));
                                self.delete_files = false;
                            }
                        }
                    });
                    if ui
                        .add_enabled(self.connected, egui::Button::new("Обновить").frame(false))
                        .clicked()
                    {
                        self.send(Command::Refresh);
                    }
                    if self.connected {
                        ui.separator();
                        ui.colored_label(
                            BLUE,
                            format!(
                                "↓ {}/с",
                                bytes(self.torrents.iter().map(|t| t.dlspeed).sum())
                            ),
                        )
                        .on_hover_text("Скорость загрузки");
                        ui.colored_label(
                            ORANGE,
                            format!(
                                "↑ {}/с",
                                bytes(self.torrents.iter().map(|t| t.upspeed).sum())
                            ),
                        )
                        .on_hover_text("Скорость отдачи");
                        ui.colored_label(
                            GREEN,
                            format!(
                                "Готово {}/{}",
                                self.torrents.iter().filter(|t| t.progress >= 1.0).count(),
                                self.torrents.len()
                            ),
                        )
                        .on_hover_text("Завершённые торренты / всего");
                    }
                });
                if !self.notice.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(
                            if self.is_error {
                                Color32::LIGHT_RED
                            } else {
                                MUTED
                            },
                            &self.notice,
                        );
                        if ui.small_button("×").clicked() {
                            self.notice.clear();
                        }
                    });
                }
                ui.add_space(8.0);
                ui.separator();
                if !self.connected {
                    ui.add_space(90.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Подключите сервер").size(22.0));
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new(
                                "Подключите сервер qBittorrent, чтобы управлять торрентами.",
                            )
                            .color(MUTED),
                        );
                        ui.add_space(16.0);
                        if ui.button("Настроить подключение").clicked() {
                            self.settings_open = true;
                        }
                    });
                    return;
                }
                let query = self.search.to_lowercase();
                let mut rows: Vec<_> = self
                    .torrents
                    .iter()
                    .filter(|t| {
                        Self::matches_filter(t, self.filter)
                            && t.name.to_lowercase().contains(&query)
                    })
                    .collect();
                rows.sort_by(|a, b| {
                    let order = crate::columns::COLUMNS[self.sort].compare(a, b);
                    if self.descending {
                        order.reverse()
                    } else {
                        order
                    }
                });
                if rows.is_empty() {
                    ui.add_space(40.0);
                    ui.label("Торренты не найдены. Добавьте торрент или измените фильтр.");
                    return;
                }
                let visible: Vec<_> = self
                    .settings
                    .visible_columns
                    .iter()
                    .filter_map(|key| {
                        crate::columns::COLUMNS
                            .iter()
                            .enumerate()
                            .find(|(_, c)| c.key == key)
                    })
                    .collect();
                let mut column_drop = None;
                let table_width = visible
                    .iter()
                    .map(|(_, c)| c.saved_width(&self.settings) + 8.0)
                    .sum::<f32>()
                    .max(ui.available_width());
                egui::ScrollArea::horizontal()
                    .id_salt("torrent-columns-scroll")
                    .show(ui, |ui| {
                        ui.set_width(table_width);
                        let keys: Vec<_> = visible.iter().map(|(_, c)| c.key).collect();
                        let layout: Vec<String> =
                            keys.iter().map(|key| (*key).to_owned()).collect();
                        let mut table = TableBuilder::new(ui)
                            .id_salt(("torrent-columns", keys))
                            .striped(true)
                            .sense(egui::Sense::click())
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center));
                        for (_, c) in &visible {
                            table = table.column(
                                Column::initial(c.saved_width(&self.settings))
                                    .at_least(60.0)
                                    .clip(true),
                            );
                        }
                        // Keyed widths are authoritative after restart or a visibility/order change.
                        if self.column_layout != layout {
                            table.reset();
                            self.column_layout = layout;
                        }
                        table
                            .header(24.0, |mut header| {
                                for (index, c) in &visible {
                                    header.col(|ui| {
                                        let title = if self.sort == *index {
                                            format!(
                                                "{} {}",
                                                c.title,
                                                if self.descending { "↓" } else { "↑" }
                                            )
                                        } else {
                                            c.title.to_owned()
                                        };
                                        let response = column_header(ui, &title);
                                        if response.drag_started() {
                                            self.dragged_column = Some(c.key.to_owned());
                                        }
                                        if let Some(source) = &self.dragged_column {
                                            if response.contains_pointer() && source != c.key {
                                                let after = ui.input(|i| {
                                                    i.pointer.hover_pos().is_some_and(|p| {
                                                        p.x > response.rect.center().x
                                                    })
                                                });
                                                let x = if after {
                                                    response.rect.right()
                                                } else {
                                                    response.rect.left()
                                                };
                                                ui.painter().vline(
                                                    x,
                                                    response.rect.y_range(),
                                                    egui::Stroke::new(2.0_f32, ACCENT),
                                                );
                                                if ui.input(|i| i.pointer.any_released()) {
                                                    column_drop = Some((
                                                        source.clone(),
                                                        c.key.to_owned(),
                                                        after,
                                                    ));
                                                }
                                            }
                                        }
                                        if response.clicked() {
                                            if self.sort == *index {
                                                self.descending = !self.descending;
                                            } else {
                                                self.sort = *index;
                                                self.descending = false;
                                            }
                                        }
                                    });
                                }
                            })
                            .body(|body| {
                                for ((_, column), width) in visible.iter().zip(body.widths()) {
                                    if width.is_finite() && *width >= 60.0 {
                                        self.settings
                                            .column_widths
                                            .insert(column.key.to_owned(), *width);
                                    }
                                }
                                body.rows(18.0, rows.len(), |mut row| {
                                    let t = rows[row.index()];
                                    row.set_selected(self.selected.as_ref() == Some(&t.hash));
                                    for (_, c) in &visible {
                                        row.col(|ui| {
                                            if c.key == "progress" {
                                                torrent_progress(ui, t);
                                            } else {
                                                let color = match c.key {
                                                    "state" => torrent_color(t),
                                                    "dlspeed" if t.dlspeed > 0 => BLUE,
                                                    "upspeed" if t.upspeed > 0 => ORANGE,
                                                    _ => ui.visuals().text_color(),
                                                };
                                                let text = c.display(t);
                                                torrent_cell_text(ui, &text, color)
                                                    .on_hover_text(text);
                                            }
                                        });
                                    }
                                    if row.response().clicked() {
                                        self.selected = Some(t.hash.clone());
                                    }
                                });
                            });
                    });
                if let Some((source, target, after)) = column_drop {
                    crate::columns::move_column(
                        &mut self.settings.visible_columns,
                        &source,
                        &target,
                        after,
                    );
                }
                if ctx.input(|i| i.pointer.any_released() || !i.pointer.primary_down()) {
                    self.dragged_column = None;
                }
                // TableBuilder applies resizing after body(); capture its final widths next frame.
                if ctx.input(|i| i.pointer.any_down() || i.pointer.any_released()) {
                    ctx.request_repaint();
                }
            });
        self.dialogs(ctx);
        self.remote.dialogs(ctx, self.connected);
        for command in self.remote.commands() {
            self.send(command);
        }
        if self.connected {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
    }
}

fn column_header(ui: &mut egui::Ui, title: &str) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
    if response.hovered() || response.dragged() {
        ui.painter()
            .rect_filled(rect, 3.0, Color32::from_rgb(36, 52, 70));
    }
    ui.painter().text(
        rect.left_center() + egui::vec2(4.0, 0.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(13.0),
        MUTED,
    );
    response
        .on_hover_cursor(egui::CursorIcon::Grab)
        .on_hover_text("Щелчок — сортировка. Перетащите заголовок для перестановки колонок.")
}

fn torrent_cell_text(ui: &mut egui::Ui, text: &str, color: Color32) -> egui::Response {
    // Selection belongs to the whole table row, not to the label's text selection.
    ui.add(
        egui::Label::new(RichText::new(text).color(color))
            .truncate()
            .selectable(false)
            .sense(egui::Sense::hover()),
    )
}

fn details_panel(
    ctx: &egui::Context,
    height: f32,
    name: &str,
    contents: impl FnOnce(&mut egui::Ui),
) -> f32 {
    egui::TopBottomPanel::bottom("details-stable")
        .resizable(true)
        .default_height(height)
        .min_height(170.0)
        .max_height((ctx.available_rect().height() - 320.0).max(170.0))
        .show(ctx, |ui| {
            ui.set_min_height(ui.available_height());
            ui.add(egui::Label::new(RichText::new(name).strong()).truncate())
                .on_hover_text("Перетащите верхнюю границу, чтобы изменить высоту панели");
            egui::ScrollArea::both()
                .id_salt("details-content")
                .auto_shrink([false, false])
                .show(ui, contents);
        })
        .response
        .rect
        .height()
}

fn navigation_row(
    ui: &mut egui::Ui,
    label: &str,
    count: usize,
    icon: usize,
    selected: bool,
) -> egui::Response {
    let color = [
        ACCENT, BLUE, ORANGE, MUTED, RED, BLUE, ACCENT, ORANGE, MUTED, BLUE, ACCENT,
    ][icon.min(10)];
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            5.0,
            if selected {
                Color32::from_rgb(32, 61, 76)
            } else {
                Color32::from_rgb(29, 43, 59)
            },
        );
    }
    if selected {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min + egui::vec2(0.0, 3.0), egui::vec2(3.0, 12.0)),
            2.0,
            color,
        );
    }
    draw_icon(
        ui.painter(),
        rect.left_center() + egui::vec2(18.0, 0.0),
        icon,
        color,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(36.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        Color32::from_rgb(226, 228, 231),
    );
    if count != usize::MAX {
        let badge = egui::Rect::from_center_size(
            rect.right_center() - egui::vec2(19.0, 0.0),
            egui::vec2(30.0, 16.0),
        );
        ui.painter()
            .rect_filled(badge, 11.0, Color32::from_rgb(40, 54, 71));
        ui.painter().text(
            badge.center(),
            egui::Align2::CENTER_CENTER,
            count,
            egui::FontId::proportional(12.0),
            color,
        );
    }
    response
}

fn action_button(label: &str, color: Color32) -> egui::Button<'_> {
    egui::Button::new(RichText::new(label).color(color).strong())
        .fill(color.gamma_multiply(0.16))
        .stroke(egui::Stroke::new(1.0_f32, color.gamma_multiply(0.4)))
        .min_size(egui::vec2(0.0, 24.0))
}

fn torrent_color(t: &Torrent) -> Color32 {
    if matches!(t.state.as_str(), "error" | "missingFiles") {
        RED
    } else if t.stopped() {
        MUTED
    } else if t.state.contains("checking") {
        Color32::from_rgb(182, 153, 255)
    } else if t.progress >= 1.0 {
        if t.upspeed > 0 {
            ORANGE
        } else {
            GREEN
        }
    } else {
        BLUE
    }
}

fn draw_icon(p: &egui::Painter, center: egui::Pos2, kind: usize, color: Color32) {
    let point = |x, y| center + egui::vec2(x, y);
    let stroke = egui::Stroke::new(1.8_f32, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([point(a.0, a.1), point(b.0, b.1)], stroke);
    };
    match kind {
        0 => {
            for x in [-6.0, 2.0] {
                for y in [-6.0, 2.0] {
                    p.rect_filled(
                        egui::Rect::from_min_size(point(x, y), egui::vec2(5.0, 5.0)),
                        1.0,
                        color,
                    );
                }
            }
        }
        1 | 2 => {
            let d = if kind == 1 { 1.0 } else { -1.0 };
            line((0.0, -7.0 * d), (0.0, 6.0 * d));
            line((-5.0, 1.0 * d), (0.0, 6.0 * d));
            line((5.0, 1.0 * d), (0.0, 6.0 * d));
        }
        3 => {
            for x in [-5.0, 2.0] {
                p.rect_filled(
                    egui::Rect::from_min_size(point(x, -6.0), egui::vec2(3.0, 12.0)),
                    1.0,
                    color,
                );
            }
        }
        4 => {
            line((-5.0, -5.0), (5.0, 5.0));
            line((-5.0, 5.0), (5.0, -5.0));
        }
        5 => {
            line((-7.0, 5.0), (-3.0, -1.0));
            line((-3.0, -1.0), (1.0, 3.0));
            line((1.0, 3.0), (7.0, -6.0));
        }
        6 => {
            p.circle_stroke(point(-2.0, -2.0), 5.0, stroke);
            line((2.0, 2.0), (7.0, 7.0));
        }
        7 => {
            p.circle_filled(point(-5.0, 5.0), 2.0, color);
            line((-5.0, -5.0), (5.0, -5.0));
            line((5.0, -5.0), (5.0, 5.0));
            line((-5.0, 0.0), (0.0, 0.0));
            line((0.0, 0.0), (0.0, 5.0));
        }
        8 => {
            for y in [-5.0, 0.0, 5.0] {
                line((-6.0, y), (6.0, y));
            }
        }
        9 => {
            for (y, x) in [(-5.0, -2.0), (1.0, 4.0), (6.0, -3.0)] {
                line((-7.0, y), (7.0, y));
                p.circle_filled(point(x, y), 2.5, color);
            }
        }
        _ => {
            line((-6.0, -5.0), (3.0, -5.0));
            line((3.0, -5.0), (7.0, 0.0));
            line((7.0, 0.0), (0.0, 7.0));
            line((0.0, 7.0), (-6.0, 1.0));
            line((-6.0, 1.0), (-6.0, -5.0));
        }
    }
}

fn torrent_progress(ui: &mut egui::Ui, torrent: &Torrent) {
    let value = torrent.progress.clamp(0.0, 1.0);
    let color = torrent_color(torrent);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 16.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 5.0, color.gamma_multiply(0.12));
    if value > 0.0 {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * value, rect.height())),
            5.0,
            color.gamma_multiply(0.42),
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{:.0}%", value * 100.0),
        egui::FontId::proportional(12.0),
        ui.visuals().text_color(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn header_blank_area_clicks_and_drag_does_not_click() {
        for drag in [false, true] {
            let ctx = egui::Context::default();
            let mut target = egui::Pos2::ZERO;
            let mut clicked = false;
            let mut dragged = false;
            let mut drop_hovered = false;
            for frame in 0..4 {
                let pos = target
                    + if drag && frame >= 2 {
                        egui::vec2(200.0, 0.0)
                    } else {
                        egui::Vec2::ZERO
                    };
                let mut input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 600.0),
                    )),
                    ..Default::default()
                };
                if frame > 0 {
                    input.events.push(egui::Event::PointerMoved(pos));
                }
                if frame == 1 || frame == 3 {
                    input.events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: frame == 1,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
                let _ = ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        TableBuilder::new(ui)
                            .resizable(true)
                            .columns(Column::initial(200.0), 2)
                            .header(24.0, |mut row| {
                                row.col(|ui| {
                                    let response = column_header(ui, "Имя");
                                    if frame == 0 {
                                        target =
                                            response.rect.right_center() - egui::vec2(30.0, 0.0);
                                    }
                                    clicked |= response.clicked();
                                    dragged |= response.drag_started();
                                });
                                row.col(|ui| {
                                    let response = column_header(ui, "Размер");
                                    if frame == 3 {
                                        drop_hovered = response.contains_pointer();
                                    }
                                });
                            })
                            .body(|_| {});
                    });
                });
            }
            assert_eq!(clicked, !drag);
            assert_eq!(dragged, drag);
            if drag {
                assert!(drop_hovered);
            }
        }
    }

    #[test]
    fn clicking_torrent_text_selects_the_row() {
        let ctx = egui::Context::default();
        let mut target = egui::Pos2::ZERO;
        let mut clicked = false;
        for frame in 0..3 {
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                ..Default::default()
            };
            if frame > 0 {
                input.events = vec![
                    egui::Event::PointerMoved(target),
                    egui::Event::PointerButton {
                        pos: target,
                        button: egui::PointerButton::Primary,
                        pressed: frame == 1,
                        modifiers: egui::Modifiers::NONE,
                    },
                ];
            }
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    TableBuilder::new(ui)
                        .sense(egui::Sense::click())
                        .column(Column::initial(400.0))
                        .body(|body| {
                            body.rows(18.0, 1, |mut row| {
                                row.col(|ui| {
                                    let response = torrent_cell_text(
                                        ui,
                                        "Clickable torrent name",
                                        Color32::WHITE,
                                    );
                                    if frame == 0 {
                                        target =
                                            response.rect.left_center() + egui::vec2(20.0, 0.0);
                                    }
                                });
                                clicked |= row.response().clicked();
                            })
                        });
                });
            });
        }
        assert!(clicked, "Clicking the text must reach the torrent row");
    }

    #[test]
    fn details_height_survives_empty_and_overflowing_tabs() {
        let ctx = egui::Context::default();
        for count in [30, 0, 1, 100, 0] {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 780.0),
                )),
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                let height = details_panel(ctx, 350.0, "Torrent", |ui| {
                    for _ in 0..count {
                        ui.label("Information");
                    }
                });
                assert!(
                    (height - 350.0).abs() <= 1.0,
                    "height changed to {height} for {count} rows"
                );
            });
        }
    }
}
