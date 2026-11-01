use crate::{
    api::{self, Command, Event},
    model::{Settings, Torrent},
};
use eframe::egui::{self, Color32, RichText};
use egui_extras::{Column, TableBuilder};
use std::sync::mpsc::{Receiver, Sender};

pub struct ClientApp {
    server_free_space: Option<u64>,
    add_flow: crate::add_torrent::AddFlow,
    pending_files: std::collections::VecDeque<std::path::PathBuf>,
    #[cfg(windows)]
    activation: Option<crate::activation::Listener>,
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
    selection: crate::selection::Selection,
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
        self.settings.theme = match std::env::var("QBIT_SMOKE_THEME").as_deref() {
            Ok("white") => crate::theme::Theme::White,
            Ok("marine") => crate::theme::Theme::Marine,
            _ => crate::theme::Theme::Graphite,
        };
        self.remote.smoke_fixture();
        if std::env::var("QBIT_SMOKE_PAGE").as_deref() == Ok("about") {
            self.about_open = true;
        }

        if std::env::var("QBIT_SMOKE_PAGE").as_deref() == Ok("path-mappings") {
            self.settings.path_mappings = vec![
                crate::path_mapping::PathMapping {
                    server: "/downloads".into(),
                    local: "Z:\\Downloads".into(),
                },
                crate::path_mapping::PathMapping {
                    server: "/media".into(),
                    local: "\\\\NAS\\Media".into(),
                },
            ];
        }
        if std::env::var("QBIT_SMOKE_PAGE").as_deref() == Ok("add-torrent") {
            self.add_flow.smoke_fixture();
        }
        self.settings.details_height = if std::env::var("QBIT_SMOKE_PAGE").as_deref() == Ok("files")
        {
            390.0
        } else {
            280.0
        };
        self.settings.close_to_tray = false;
        self.settings.zoom = std::env::var("QBIT_SMOKE_ZOOM")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1.0);
        self.connected = true;
        self.server_free_space = Some(
            std::env::var("QBIT_SMOKE_FREE_SPACE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(348 * 1024 * 1024 * 1024),
        );
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
        self.selection.hashes.insert("fixture-1".into());
        if std::env::var_os("QBIT_SMOKE_MULTI").is_some() {
            self.selection.hashes.insert("fixture-2".into());
        }
        #[cfg(windows)]
        assert!(self.tray.is_some(), "Windows tray creation failed");
    }
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = &cc.egui_ctx;
        ctx.set_theme(egui::Theme::Dark);

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
            let mut wordmark = vec!["segoe".to_owned()];
            if let Ok(font) = std::fs::read("C:/Windows/Fonts/seguisb.ttf") {
                fonts.font_data.insert(
                    "brand-semibold".into(),
                    egui::FontData::from_owned(font).into(),
                );
                wordmark.insert(0, "brand-semibold".into());
            }
            fonts
                .families
                .insert(egui::FontFamily::Name("qbit-wordmark".into()), wordmark);
            ctx.data_mut(|data| data.insert_temp(egui::Id::new("brand-font"), true));
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
        settings.theme.apply(ctx);
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
            server_free_space: None,
            add_flow: Default::default(),
            pending_files: Default::default(),
            #[cfg(windows)]
            activation: None,
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
            selection: Default::default(),
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
        #[cfg(windows)]
        if !cfg!(feature = "ui-smoke") {
            match crate::activation::Listener::start(ctx.clone()) {
                Ok(listener) => app.activation = Some(listener),
                Err(error) => {
                    app.notice = format!("Приём файлов из Windows недоступен: {error}");
                    app.is_error = true;
                }
            }
        }
        app
    }
    pub fn open_files(&mut self, files: Vec<std::path::PathBuf>) {
        if files.is_empty() {
            return;
        }
        for file in files {
            if !self.pending_files.contains(&file) {
                self.pending_files.push_back(file);
            }
        }
        self.remote.page = crate::remote::Page::Torrents;
        if !self.connected && !self.connecting {
            if self.settings.servers.is_empty() {
                self.settings_open = true;
            } else {
                self.connect();
            }
        }
    }
    fn receive_activation(&mut self, ctx: &egui::Context) {
        #[cfg(windows)]
        while let Some(args) = self.activation.as_ref().and_then(|a| a.rx.try_recv().ok()) {
            if let Some(tray) = &self.tray {
                tray.control.restore();
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            match crate::activation::files(args) {
                Ok(files) => self.open_files(files),
                Err(error) => {
                    self.notice = error;
                    self.is_error = true;
                }
            }
        }
        if self.connected {
            while let Some(file) = self.pending_files.pop_front() {
                self.add_flow
                    .push(crate::add_torrent::Source::File(file), String::new());
            }
        }
    }
    fn with_tray_notice(mut self, error: Option<String>) -> Self {
        if let Some(error) = error {
            self.notice = error;
            self.is_error = true;
        }
        self
    }
    fn selected_hashes(&self) -> String {
        self.torrents
            .iter()
            .filter(|t| {
                self.selection.hashes.contains(&t.hash)
                    && Self::matches_filter(t, self.filter)
                    && t.name.to_lowercase().contains(&self.search.to_lowercase())
            })
            .map(|t| t.hash.clone())
            .collect::<Vec<_>>()
            .join("|")
    }
    fn selected_names(&self) -> String {
        let hashes = self.selected_hashes();
        self.torrents
            .iter()
            .filter(|t| hashes.split('|').any(|h| h == t.hash))
            .map(|t| t.name.clone())
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn prepare_delete(&mut self) {
        let hashes = self.selected_hashes();
        if hashes.is_empty() {
            return;
        }
        self.delete = Some((
            hashes.clone(),
            format!(
                "Торрентов: {}\n{}",
                hashes.split('|').count(),
                self.selected_names()
            ),
        ));
        self.delete_files = false;
    }
    fn send(&mut self, command: Command) {
        if self.tx.send(command).is_err() {
            self.notice = "Сетевой поток завершился. Перезапустите приложение.".into();
            self.is_error = true;
        }
    }
    fn connect(&mut self) {
        self.server_free_space = None;
        self.add_flow.reset_session();
        self.settings.save_server();
        self.connection_epoch += 1;
        self.remote.clear();
        self.delete = None;
        self.connecting = true;
        self.connected = false;
        self.torrents.clear();
        self.selected = None;
        self.selection.clear();
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
        self.server_free_space = None;
        self.add_flow.reset_session();
        self.connection_epoch += 1;
        self.send(Command::Disconnect(self.connection_epoch));
        self.connected = false;
        self.connecting = false;
        self.remote.clear();
        self.torrents.clear();
        self.selected = None;
        self.selection.clear();
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
                Event::ServerSpace(free) => self.server_free_space = free,
                Event::TorrentPrepared { id, result } => self.add_flow.prepared(id, result),
                Event::TorrentAdded { id, result } => {
                    if let Some(result) = self.add_flow.added(id, result) {
                        match result {
                            Ok(message) => {
                                self.notice = message;
                                self.is_error = false;
                            }
                            Err(error) => {
                                self.notice = error;
                                self.is_error = true;
                            }
                        }
                    }
                }
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
                    self.server_free_space = None;
                    self.add_flow.reset_session();
                    self.remote.clear();
                    self.delete = None;
                    self.connected = false;
                    self.connecting = false;
                    self.torrents.clear();
                    self.selected = None;
                    self.selection.clear();
                }
                Event::Torrents(torrents) => {
                    self.torrents = torrents;
                    let available = self
                        .torrents
                        .iter()
                        .map(|t| t.hash.clone())
                        .collect::<Vec<_>>();
                    self.selection.retain(&available);
                    if self
                        .selected
                        .as_ref()
                        .is_some_and(|h| !self.selection.hashes.contains(h))
                    {
                        self.selected = self.selection.hashes.iter().next().cloned();
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
        if !self.connected && !self.pending_files.is_empty() {
            egui::Window::new("Торренты ожидают подключения").collapsible(false).default_width(450.0).show(ctx,|ui|{
                ui.label(format!("Файлов: {} · сервер: {}",self.pending_files.len(),self.settings.server_name));
                ui.label("После подключения откроется диалог выбора файлов. Добавление на сервер выполняется только после подтверждения.");
                egui::ScrollArea::vertical().max_height(150.0).show(ui,|ui|{for file in &self.pending_files{ui.label(file.file_name().unwrap_or_default().to_string_lossy());}});
                if self.connecting {ui.spinner();}else if ui.button("Настроить подключение").clicked(){self.settings_open=true;}
                if ui.button("Отменить добавление файлов").clicked(){self.pending_files.clear();}
            });
        }
        let palette = crate::theme::palette(ctx);
        egui::Window::new("О программе")
            .open(&mut self.about_open)
            .collapsible(false)
            .resizable(false)
            .default_width(560.0)
            .max_width((ctx.screen_rect().width() - 48.0).clamp(280.0, 560.0))
            .max_height((ctx.screen_rect().height() - 64.0).max(240.0))
            .vscroll(true)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    crate::icon::presentation(ui);
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Версия {}", env!("CARGO_PKG_VERSION"))).strong(),
                    );
                    ui.label("©2026, Copyleft Yaroslav Blohin");
                    ui.label("Клиент для удалённого управления qBittorrent.");
                });
                ui.separator();
                ui.collapsing("Управление интерфейсом", |ui| {
                    ui.label("Щелчок по заголовку — сортировка.");
                    ui.label("Перетаскивание заголовка — порядок колонок.");
                    ui.label("Перетаскивание границы — ширина колонки.");
                    ui.label("Вид → Колонки — выбор отображаемых полей.");
                });
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
                        self.settings.path_mappings.clear();
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
                            self.settings.path_mappings.clear();
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
                ui.label(RichText::new("Включите Web UI в настройках сервера.").color(palette.muted));
                ui.small("Адрес, логин и пароль можно изменить здесь. Их изменение отключает текущий сервер.");
                ui.add_enabled_ui(!self.connecting, |ui| {
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
                    let password_changed = ui.add(egui::TextEdit::singleline(&mut self.password).password(true)).changed();
                    if address_changed || username_changed || password_changed {
                        self.settings.saved_password = None;
                        if self.connected {
                            self.disconnect();
                            self.notice = "Параметры подключения изменены. Подключитесь с новыми данными.".into();
                            self.is_error = false;
                        }
                    }
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
                crate::path_mapping::settings_ui(ui, &mut self.settings.path_mappings);
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
                    RichText::new("Выход — через меню «Файл» или значок в трее.").color(palette.muted),
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
                        for url in self.links.lines().map(str::trim).filter(|s| !s.is_empty()) {
                            self.add_flow.push(
                                crate::add_torrent::Source::Link {
                                    url: url.into(),
                                    downloader: String::new(),
                                },
                                self.save_path.clone(),
                            );
                        }
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
                            self.add_flow.push(
                                crate::add_torrent::Source::File(file),
                                self.save_path.clone(),
                            );
                            self.add_open = false;
                        }
                    }
                });
            });
        self.add_open = add_open && self.add_open;
        if let Some((hash, name)) = self.delete.clone() {
            egui::Window::new("Удалить выбранные торренты?")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(220.0)
                        .show(ui, |ui| {
                            ui.label(name);
                        });
                    ui.checkbox(&mut self.delete_files, "Также удалить файлы на сервере");
                    if self.delete_files {
                        ui.colored_label(palette.red, "Файлы будут удалены с удалённого сервера.");
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
                    let hashes = self.selected_hashes();
                    ui.add_enabled_ui(self.connected && !hashes.is_empty(), |ui| {
                        if ui.button("Запустить выбранные").clicked() {
                            self.send(Command::Start(hashes.clone()));
                            ui.close_menu();
                        }
                        if ui.button("Остановить выбранные").clicked() {
                            self.send(Command::Stop(hashes.clone()));
                            ui.close_menu();
                        }
                        if ui.button("Удалить выбранные…").clicked() {
                            self.prepare_delete();
                            ui.close_menu();
                        }
                        if ui.button("Копировать названия").clicked() {
                            ctx.copy_text(self.selected_names());
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
                    ui.menu_button("Тема оформления", |ui| {
                        for theme in crate::theme::Theme::ALL {
                            if ui
                                .selectable_value(&mut self.settings.theme, theme, theme.name())
                                .changed()
                            {
                                ctx.request_repaint();
                                ui.close_menu();
                            }
                        }
                    });
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
        if crate::theme::current(ctx) != self.settings.theme {
            self.settings.theme.apply(ctx);
        }
        let palette = self.settings.theme.palette();
        #[cfg(feature = "ui-smoke")]
        ctx.set_zoom_factor(self.settings.zoom);
        self.receive();
        self.receive_activation(ctx);
        if let Some(command) = self.add_flow.tick(self.connected) {
            self.send(command);
        }
        // Mirror native tray changes into eframe's cached viewport visibility.
        #[cfg(windows)]
        if let Some(visible) = self
            .tray
            .as_ref()
            .and_then(|tray| tray.control.take_visibility())
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(visible));
            if visible {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            }
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
        crate::status_bar::show(
            ctx,
            &crate::status_bar::State {
                connected: self.connected,
                connecting: self.connecting,
                server: &self.settings.server_name,
                version: &self.version,
                download: self.torrents.iter().map(|t| t.dlspeed).sum(),
                upload: self.torrents.iter().map(|t| t.upspeed).sum(),
                free: self.server_free_space,
                selected: self.selection.hashes.len(),
            },
        );
        egui::SidePanel::left("sidebar")
            .exact_width(218.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(palette.sidebar).inner_margin(14.0))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("navigation")
                    .show(ui, |ui| {
                        ui.add_space(8.0);
                        crate::icon::header(ui);
                        ui.add_space(12.0);
                        ui.label(RichText::new("СЕРВЕР").size(10.0).color(palette.muted));
                        ui.add(
                            egui::Label::new(
                                RichText::new(&self.settings.server_name).color(palette.accent),
                            )
                            .truncate(),
                        )
                        .on_hover_text("Переключение и управление — в меню «Серверы»");
                        ui.add_space(24.0);
                        ui.label(RichText::new("ТОРРЕНТЫ").size(10.0).color(palette.muted));
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
                        ui.label(RichText::new("ИНСТРУМЕНТЫ").size(10.0).color(palette.muted));
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
                    });
            });
        if self.remote.page == crate::remote::Page::Torrents {
            if let Some(t) = self
                .torrents
                .iter()
                .find(|t| Some(&t.hash) == self.selected.as_ref())
            {
                let (height, close) =
                    details_panel(ctx, self.settings.details_height, &t.name, |ui| {
                        self.remote.details(ui, t, &self.settings.path_mappings);
                    });
                self.settings.details_height = height;
                if close {
                    self.selection.clear();
                    self.selected = None;
                }
            }
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(palette.surface).inner_margin(16.0))
            .show(ctx, |ui| {
                if !self.remote.message.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(
                            if self.remote.error {
                                palette.red
                            } else {
                                palette.muted
                            },
                            &self.remote.message,
                        );
                        if ui.small_button("×").clicked() {
                            self.remote.message.clear();
                        }
                    });
                }
                if self.connected && self.remote.page != crate::remote::Page::Torrents {
                    self.remote.page_ui(ui, &mut self.settings.path_mappings);
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
                    ui.label(RichText::new(self.torrents.len().to_string()).color(palette.muted));

                });
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    ui.add_sized(egui::vec2(180.0, 32.0),
                        egui::TextEdit::singleline(&mut self.search)
                            .hint_text("Поиск торрентов…")
                            .margin(egui::Margin::symmetric(8, 7))
                            .desired_width(180.0),
                    );
                    if ui.add_enabled(self.connected, |ui: &mut egui::Ui| torrent_action_button(ui, "Добавить торрент", 3, Color32::from_rgb(30, 125, 205))).clicked() {
                        self.add_open = true;
                    }
                    let hashes = self.selected_hashes();
                    ui.add_enabled_ui(self.connected && !hashes.is_empty(), |ui| {
                        if ui
                            .add(|ui: &mut egui::Ui| torrent_action_button(ui, "Запустить", 0, Color32::from_rgb(24, 164, 66)))
                            .clicked()
                        {
                            self.send(Command::Start(hashes.clone()));
                        }
                        if ui
                            .add(|ui: &mut egui::Ui| torrent_action_button(ui, "Остановить", 1, Color32::from_rgb(184, 109, 18)))
                            .clicked()
                        {
                            self.send(Command::Stop(hashes.clone()));
                        }
                        if ui.add(|ui: &mut egui::Ui| torrent_action_button(ui, "Удалить", 2, Color32::from_rgb(205, 47, 54))).clicked() {
                            self.prepare_delete();
                        }
                        for (label, icon, up, color) in [
                            ("Выше в очереди", 4, true, Color32::from_rgb(133, 76, 199)),
                            ("Ниже в очереди", 5, false, Color32::from_rgb(136, 36, 66)),
                        ] {
                            if ui.add(|ui: &mut egui::Ui| torrent_action_button(ui, label, icon, color)).clicked() {
                                self.remote.move_in_queue(&hashes, up);
                            }
                        }
                    });


                });
                if !self.notice.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.colored_label(
                            if self.is_error {
                                palette.red
                            } else {
                                palette.muted
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
                            .color(palette.muted),
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
                let ordered_hashes: Vec<String> = rows.iter().map(|t| t.hash.clone()).collect();
                self.selection.retain(&ordered_hashes);
                if self
                    .selected
                    .as_ref()
                    .is_some_and(|h| !self.selection.hashes.contains(h))
                {
                    self.selected = self.selection.hashes.iter().next().cloned();
                }
                if !ctx.wants_keyboard_input() && !self.add_flow.active()
                    && !self.remote.dialog_open()
                    && !ctx.memory(|m| m.any_popup_open())
                    && !self.settings_open
                    && !self.add_open
                    && self.delete.is_none()
                {
                    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::A)) {
                        self.selection.all(&ordered_hashes);
                        self.selected = ordered_hashes.first().cloned();
                    }
                    if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                        self.selection.clear();
                        self.selected = None;
                    }
                }
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
                                                    egui::Stroke::new(2.0_f32, palette.accent),
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
                                    row.set_selected(self.selection.hashes.contains(&t.hash));
                                    for (_, c) in &visible {
                                        row.col(|ui| {
                                            if c.key == "progress" {
                                                torrent_progress(ui, t);
                                            } else {
                                                let color = match c.key {
                                                    "state" => torrent_color(t, palette),
                                                    "dlspeed" if t.dlspeed > 0 => palette.blue,
                                                    "upspeed" if t.upspeed > 0 => palette.green,
                                                    _ => ui.visuals().text_color(),
                                                };
                                                let text = c.display(t);
                                                torrent_cell_text(ui, &text, color)
                                                    .on_hover_text(text);
                                            }
                                        });
                                    }
                                    let response = row.response();
                                    #[cfg(feature = "ui-smoke")]
                                    if t.hash == "fixture-1" {
                                        ctx.data_mut(|data| {
                                            data.insert_temp(
                                                egui::Id::new("smoke-torrent-row"),
                                                response.rect.center(),
                                            )
                                        });
                                    }
                                    if response.clicked() || response.secondary_clicked() {
                                        let modifiers = ctx.input(|i| i.modifiers);
                                        self.selection.click(
                                            &t.hash,
                                            &ordered_hashes,
                                            modifiers.ctrl,
                                            modifiers.shift,
                                            response.secondary_clicked(),
                                        );
                                        self.selected = if self.selection.hashes.contains(&t.hash) {
                                            Some(t.hash.clone())
                                        } else {
                                            self.selection.hashes.iter().next().cloned()
                                        };
                                    }
                                    response.context_menu(|ui| {
                                        ui.set_width(290.0);
                                        ui.style_mut().override_font_id=Some(egui::FontId::proportional(12.0));
                                        ui.spacing_mut().item_spacing.y=1.0;
                                        egui::ScrollArea::vertical()
                                            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                                            .max_height((ctx.screen_rect().height()*0.8).max(180.0))
                                            .show(ui,|ui| {
                                        let hashes = self
                                            .selection
                                            .hashes
                                            .iter()
                                            .cloned()
                                            .collect::<Vec<_>>()
                                            .join("|");
                                        let count = self.selection.hashes.len();
                                        ui.weak(format!("Выбрано: {count}"));
                                        if crate::remote::action_menu_item(ui,"Запустить",crate::ui_icons::PLAY,false).clicked() {
                                            let _ = self.tx.send(Command::Start(hashes.clone()));
                                            ui.close_menu();
                                        }
                                        if crate::remote::action_menu_item(ui,"Остановить",3,false).clicked() {
                                            let _ = self.tx.send(Command::Stop(hashes.clone()));
                                            ui.close_menu();
                                        }
                                        if crate::remote::action_menu_item(ui,"Удалить…",4,false).clicked() {
                                            let names = rows
                                                .iter()
                                                .filter(|t| self.selection.hashes.contains(&t.hash))
                                                .map(|t| t.name.clone())
                                                .collect::<Vec<_>>()
                                                .join("\n");
                                            self.delete = Some((
                                                hashes.clone(),
                                                format!("Торрентов: {count}\n{names}"),
                                            ));
                                            self.delete_files = false;
                                            ui.close_menu();
                                        }

                                        self.remote.torrent_actions(ui, t, &hashes, count, &self.settings.path_mappings);

                                        });
                                    });
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
        for source in std::mem::take(&mut self.remote.add_sources) {
            self.add_flow.push(source, String::new());
        }
        if let Some(command) = self
            .add_flow
            .ui(ctx, self.connected, &self.settings.server_name)
        {
            self.send(command);
        }
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
    let palette = crate::theme::palette(ui.ctx());
    let (rect, response) =
        ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
    if response.hovered() || response.dragged() {
        ui.painter().rect_filled(rect, 3.0, palette.hover);
    }
    ui.painter().text(
        rect.left_center() + egui::vec2(4.0, 0.0),
        egui::Align2::LEFT_CENTER,
        title,
        egui::FontId::proportional(13.0),
        palette.muted,
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
) -> (f32, bool) {
    let mut close = false;
    let panel = egui::TopBottomPanel::bottom("details-stable")
        .resizable(true)
        .default_height(height)
        .min_height(170.0)
        .max_height((ctx.available_rect().height() - 320.0).max(170.0))
        .show(ctx, |ui| {
            ui.set_min_height(ui.available_height());
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2((ui.available_width() - 30.0).max(0.0), 22.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.add(egui::Label::new(RichText::new(name).strong()).truncate())
                            .on_hover_text(
                                "Перетащите верхнюю границу, чтобы изменить высоту панели",
                            );
                    },
                );
                let response =
                    ui.add_sized(egui::vec2(22.0, 22.0), egui::Button::new("").frame(false));
                let center = response.rect.center();
                let color = if response.hovered() {
                    crate::theme::palette(ui.ctx()).text
                } else {
                    crate::theme::palette(ui.ctx()).muted
                };
                for sign in [-1.0, 1.0] {
                    ui.painter().line_segment(
                        [
                            center + egui::vec2(-4.0, -4.0 * sign),
                            center + egui::vec2(4.0, 4.0 * sign),
                        ],
                        egui::Stroke::new(1.5_f32, color),
                    );
                }
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        true,
                        "Скрыть панель информации",
                    )
                });
                close = response
                    .on_hover_text("Скрыть панель информации (Esc)")
                    .clicked();
            });
            egui::ScrollArea::both()
                .id_salt("details-content")
                .auto_shrink([false, false])
                .show(ui, contents);
        });
    (panel.response.rect.height(), close)
}

fn navigation_row(
    ui: &mut egui::Ui,
    label: &str,
    count: usize,
    icon: usize,
    selected: bool,
) -> egui::Response {
    let palette = crate::theme::palette(ui.ctx());
    let color = crate::ui_icons::color(icon, palette);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 18.0), egui::Sense::click());
    if selected || response.hovered() {
        ui.painter().rect_filled(
            rect,
            5.0,
            if selected {
                palette.selection
            } else {
                palette.hover
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
        if icon == 4 {
            crate::ui_icons::ERROR
        } else {
            icon
        },
        color,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(36.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(13.0),
        palette.text,
    );
    if count != usize::MAX {
        let badge = egui::Rect::from_center_size(
            rect.right_center() - egui::vec2(19.0, 0.0),
            egui::vec2(30.0, 16.0),
        );
        ui.painter().rect_filled(badge, 11.0, palette.badge);
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

fn pause_bars(center: egui::Pos2, pixels_per_point: f32) -> [egui::Rect; 2] {
    let width = (3.0 * pixels_per_point).round().max(1.0);
    let gap = (3.0 * pixels_per_point).round().max(1.0);
    let height = (12.0 * pixels_per_point).round().max(1.0);
    let left = (center.x * pixels_per_point - (2.0 * width + gap) / 2.0).round();
    let top = (center.y * pixels_per_point - height / 2.0).round();
    [0.0, width + gap].map(|offset| {
        egui::Rect::from_min_size(
            egui::pos2((left + offset) / pixels_per_point, top / pixels_per_point),
            egui::vec2(width / pixels_per_point, height / pixels_per_point),
        )
    })
}

fn torrent_action_button(
    ui: &mut egui::Ui,
    label: &str,
    icon: u8,
    color: Color32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
    let enabled = ui.is_enabled();
    let fill = if !enabled {
        color.gamma_multiply(0.35)
    } else if response.is_pointer_button_down_on() {
        color.gamma_multiply(0.8)
    } else if response.hovered() {
        color.linear_multiply(1.12)
    } else {
        color
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 4.0, fill);
    let center = rect.center();
    let foreground = if enabled {
        Color32::WHITE
    } else {
        Color32::from_gray(155)
    };
    let pixels_per_point = ui.ctx().pixels_per_point();
    let snap = |p: egui::Pos2| {
        egui::pos2(
            (p.x * pixels_per_point).round() / pixels_per_point,
            (p.y * pixels_per_point).round() / pixels_per_point,
        )
    };
    match icon {
        0 => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    snap(center + egui::vec2(-4.0, -6.0)),
                    snap(center + egui::vec2(-4.0, 6.0)),
                    snap(center + egui::vec2(6.0, 0.0)),
                ],
                foreground,
                egui::Stroke::NONE,
            ));
        }
        1 => {
            for bar in pause_bars(center, pixels_per_point) {
                painter.rect_filled(bar, 0.0, foreground);
            }
        }
        3 => {
            let thickness = (2.0 * pixels_per_point).round().max(1.0);
            let length = (12.0 * pixels_per_point).round().max(3.0);
            for size in [egui::vec2(length, thickness), egui::vec2(thickness, length)] {
                let min = egui::pos2(
                    (center.x * pixels_per_point - size.x / 2.0).round() / pixels_per_point,
                    (center.y * pixels_per_point - size.y / 2.0).round() / pixels_per_point,
                );
                painter.rect_filled(
                    egui::Rect::from_min_size(min, size / pixels_per_point),
                    0.0,
                    foreground,
                );
            }
        }
        4 | 5 => {
            let direction = if icon == 4 { -1.0 } else { 1.0 };
            let stem_min = snap(center + egui::vec2(-1.0, -2.0));
            let stem_max = snap(center + egui::vec2(1.0, 2.0));
            let tail = snap(center + egui::vec2(0.0, -6.0 * direction));
            painter.rect_filled(
                egui::Rect::from_min_max(
                    egui::pos2(stem_min.x, stem_min.y.min(tail.y)),
                    egui::pos2(stem_max.x, stem_max.y.max(tail.y)),
                ),
                0.0,
                foreground,
            );
            painter.add(egui::Shape::convex_polygon(
                vec![
                    snap(center + egui::vec2(0.0, 7.0 * direction)),
                    snap(center + egui::vec2(-5.0, direction)),
                    snap(center + egui::vec2(5.0, direction)),
                ],
                foreground,
                egui::Stroke::NONE,
            ));
        }
        _ => {
            for sign in [-1.0, 1.0] {
                painter.line_segment(
                    [
                        center + egui::vec2(-4.5, -4.5 * sign),
                        center + egui::vec2(4.5, 4.5 * sign),
                    ],
                    egui::Stroke::new(2.0_f32, foreground),
                );
            }
        }
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(label)
}

fn torrent_color(t: &Torrent, palette: crate::theme::Palette) -> Color32 {
    if matches!(t.state.as_str(), "error" | "missingFiles") {
        palette.red
    } else if t.stopped() {
        palette.muted
    } else if t.state.contains("checking") {
        palette.purple
    } else if t.progress >= 1.0 {
        palette.green
    } else {
        palette.blue
    }
}

pub(crate) fn draw_icon(p: &egui::Painter, center: egui::Pos2, kind: usize, color: Color32) {
    crate::ui_icons::draw(p, center, kind, color);
}

fn torrent_progress(ui: &mut egui::Ui, torrent: &Torrent) {
    let value = torrent.progress.clamp(0.0, 1.0);
    let color = torrent_color(torrent, crate::theme::palette(ui.ctx()));
    let fill_strength =
        if crate::theme::current(ui.ctx()) == crate::theme::Theme::Graphite && !torrent.stopped() {
            0.62
        } else {
            0.42
        };
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 16.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 5.0, color.gamma_multiply(0.12));
    if value > 0.0 {
        ui.painter().rect_filled(
            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * value, rect.height())),
            5.0,
            color.gamma_multiply(fill_strength),
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
        for button in [egui::PointerButton::Primary, egui::PointerButton::Secondary] {
            let ctx = egui::Context::default();
            let mut menu_opened = false;
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
                            button,
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
                                    let response = row.response();
                                    clicked |= response.clicked() || response.secondary_clicked();
                                    response.context_menu(|ui| {
                                        menu_opened = true;
                                        ui.label("Torrent actions");
                                    });
                                })
                            });
                    });
                });
            }
            assert!(clicked, "Clicking the text must reach the torrent row");
            assert_eq!(menu_opened, button == egui::PointerButton::Secondary);
        }
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
                let (height, close) = details_panel(ctx, 350.0, "Torrent", |ui| {
                    for _ in 0..count {
                        ui.label("Information");
                    }
                });
                assert!(!close);
                assert!(
                    (height - 350.0).abs() <= 1.0,
                    "height changed to {height} for {count} rows"
                );
            });
        }
    }
}

#[cfg(test)]
mod pause_icon_tests {
    use super::*;
    #[test]
    fn pause_bars_have_identical_integer_pixel_bounds_at_fractional_scales() {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0] {
            for offset in [0.0, 0.2, 0.5, 0.75] {
                let bars = pause_bars(egui::pos2(123.0 + offset, 51.0 + offset), scale);
                assert!((bars[0].width() - bars[1].width()).abs() < 0.001);
                assert_eq!(bars[0].top(), bars[1].top());
                assert_eq!(bars[0].bottom(), bars[1].bottom());
                assert!(bars[0].right() < bars[1].left());
                for bar in bars {
                    for edge in [bar.left(), bar.right(), bar.top(), bar.bottom()] {
                        assert!((edge * scale - (edge * scale).round()).abs() < 0.001);
                    }
                }
            }
        }
    }
}
