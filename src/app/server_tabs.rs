use super::{api, egui, ClientApp, Settings, TabAction};

pub struct TabbedApp {
    current: ClientApp,
    background: Vec<ClientApp>,
    next_tab_id: u64,
    reveal_active: bool,
}

impl TabbedApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        #[cfg(target_os = "macos")]
        crate::macos_app::set_window(cc);
        let mut app = Self {
            current: ClientApp::new(cc),
            background: Vec::new(),
            next_tab_id: 1,
            reveal_active: false,
        };
        app.restore_background(&cc.egui_ctx);
        app
    }

    fn restore_background(&mut self, ctx: &egui::Context) {
        let tabs = self.current.settings.startup_server_tabs();
        let auto_connect =
            self.current.settings.auto_connect && !cfg!(all(windows, feature = "ui-smoke"));
        for (position, index) in tabs.iter().copied().enumerate() {
            if index == self.current.settings.selected_server {
                self.current.tab_id = position as u64;
                continue;
            }
            let mut settings = Settings::default();
            settings.apply_profile(self.current.settings.servers[index].clone());
            let mut session = self.make_session(ctx, index, settings);
            session.tab_id = position as u64;
            if auto_connect && !session.is_error {
                session.begin_connection();
            }
            self.background.push(session);
        }
        self.next_tab_id = self.next_tab_id.max(tabs.len() as u64);
        self.reveal_active = true;
    }

    fn remember_open_tabs(&mut self) {
        let profile_count = self.current.settings.servers.len();
        let mut tabs: Vec<_> = std::iter::once(&self.current)
            .chain(&self.background)
            .map(|tab| (tab.tab_id, tab.settings.selected_server))
            .filter(|(_, index)| *index < profile_count)
            .collect();
        tabs.sort_unstable_by_key(|(id, _)| *id);
        self.current.settings.open_server_tabs =
            Some(tabs.into_iter().map(|(_, index)| index).collect());
    }

    #[cfg(windows)]
    pub fn start_hidden(&self, ctx: &egui::Context) {
        self.current.start_hidden(ctx);
    }

    pub fn open_sources(&mut self, sources: Vec<crate::add_torrent::Source>) {
        self.current.queue_additions(sources);
    }

    #[cfg(all(feature = "ui-smoke", windows))]
    pub fn smoke_tray_control(&mut self) -> crate::tray::WindowControl {
        self.current.smoke_tray_control()
    }

    #[cfg(all(feature = "ui-smoke", windows))]
    pub fn smoke_fixture(&mut self, ctx: &egui::Context) {
        self.current.smoke_fixture();
        if std::env::var("QBIT_SMOKE_PAGE").as_deref() == Ok("server-tabs") {
            self.current.settings.save_server();
            let settings = Settings {
                server_name: "Медиасервер".into(),
                address: "http://media.example.test:8080".into(),
                ..Default::default()
            };
            let index = self.current.settings.servers.len();
            self.current.settings.servers.push(settings.profile());
            let mut session = self.make_session(ctx, index, settings);
            session.connected = true;
            self.background.push(session);
        }
    }

    fn sync_profiles(&mut self) {
        let current = self.current.profile_snapshot();
        let settings = &mut self.current.settings;
        if settings.selected_server < settings.servers.len() {
            settings.servers[settings.selected_server] = current;
        }
        for session in &self.background {
            if let Some(profile) = settings.servers.get_mut(session.settings.selected_server) {
                *profile = session.profile_snapshot();
            }
        }
    }

    fn make_session(&mut self, ctx: &egui::Context, index: usize, settings: Settings) -> ClientApp {
        let (tx, rx) = api::worker(ctx.clone());
        let password = crate::credentials::load(&settings);
        let (sort, descending) = crate::columns::restored_sort(&settings);
        let mut session = ClientApp::session(settings, String::new(), tx, rx, sort, descending);
        session.settings.selected_server = index;
        session.tab_id = self.next_tab_id;
        self.next_tab_id += 1;
        match password {
            Ok(password) => session.password = password,
            Err(error) => {
                session.notice = error;
                session.is_error = true;
                session.settings_open = true;
            }
        }
        session
    }

    // The profile catalog and native integrations live with the foreground session.
    // Each tab keeps its own worker, profile fields, and UI state.
    fn replace_current(&mut self, ctx: &egui::Context, mut next: ClientApp) -> ClientApp {
        let settings = &mut self.current.settings;
        next.settings.servers = std::mem::take(&mut settings.servers);
        next.settings.profiles_initialized = settings.profiles_initialized;
        next.settings.visible_columns = settings.visible_columns.clone();
        next.settings.column_widths = settings.column_widths.clone();
        next.settings.theme = settings.theme;
        next.settings.zoom = settings.zoom;
        next.settings.close_to_tray = settings.close_to_tray;
        next.settings.auto_connect = settings.auto_connect;
        #[cfg(windows)]
        {
            next.activation = self.current.activation.take();
        }
        #[cfg(any(windows, target_os = "macos"))]
        {
            next.tray = self.current.tray.take();
        }
        self.reveal_active = true;
        ctx.memory_mut(|memory| {
            memory.close_popup();
            if let Some(id) = memory.focused() {
                memory.surrender_focus(id);
            }
        });
        ctx.request_repaint();
        std::mem::replace(&mut self.current, next)
    }

    fn open(&mut self, ctx: &egui::Context, index: usize) {
        if index == self.current.settings.selected_server {
            return;
        }
        self.sync_profiles();
        let Some(profile) = self.current.settings.servers.get(index).cloned() else {
            return;
        };
        let parked = self
            .background
            .iter()
            .position(|tab| tab.settings.selected_server == index);
        let next = if let Some(slot) = parked {
            self.background.remove(slot)
        } else {
            let mut settings = Settings::default();
            settings.apply_profile(profile);
            self.make_session(ctx, index, settings)
        };
        let mut old = self.replace_current(ctx, next);
        if old.settings.selected_server < self.current.settings.servers.len() {
            self.background.push(old);
        } else {
            self.current
                .queue_additions(old.pending_additions.drain(..));
        }
        if parked.is_none() && !self.current.is_error && !self.current.connecting {
            self.current.connect();
        }
    }

    fn new_profile(&mut self, ctx: &egui::Context) {
        self.sync_profiles();
        let catalog = &mut self.current.settings.servers;
        let index = catalog.len();
        let settings = Settings {
            server_name: format!("Сервер {}", index + 1),
            ..Default::default()
        };
        let profile = settings.profile();
        if self.current.settings.selected_server >= index {
            self.current.settings.server_name = profile.name;
            self.current.settings.save_server();
        } else {
            catalog.push(profile);
            let next = self.make_session(ctx, index, settings);
            let old = self.replace_current(ctx, next);
            self.background.push(old);
        }
        self.current.settings_open = true;
        self.reveal_active = true;
        ctx.request_repaint();
    }

    fn close(&mut self, ctx: &egui::Context, index: usize) {
        self.sync_profiles();
        if self.current.settings.selected_server != index {
            self.background
                .retain(|tab| tab.settings.selected_server != index);
            ctx.request_repaint();
            return;
        }
        let next = if let Some((slot, _)) = self
            .background
            .iter()
            .enumerate()
            .min_by_key(|(_, tab)| tab.tab_id)
        {
            self.background.remove(slot)
        } else {
            let index = self.current.settings.servers.len();
            self.make_session(ctx, index, Settings::default())
        };
        self.replace_current(ctx, next);
    }

    fn delete_profile(&mut self, ctx: &egui::Context) {
        let index = self.current.settings.selected_server;
        if index >= self.current.settings.servers.len() {
            return;
        }
        self.close(ctx, index);
        self.current.settings.servers.remove(index);
        for tab in std::iter::once(&mut self.current).chain(&mut self.background) {
            if tab.settings.selected_server > index {
                tab.settings.selected_server -= 1;
            }
        }
        self.current.settings_open = true;
    }

    fn show_tabs(&mut self, ctx: &egui::Context) -> Option<TabAction> {
        let mut action = None;
        let palette = crate::theme::palette(ctx);
        let mut tabs: Vec<_> = std::iter::once(&self.current)
            .chain(&self.background)
            .collect();
        tabs.sort_by_key(|tab| tab.tab_id);
        let reveal = std::mem::take(&mut self.reveal_active);
        egui::TopBottomPanel::top("server-tabs")
            .frame(
                egui::Frame::new()
                    .fill(palette.surface)
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt("open-server-tabs")
                        .max_width((ui.available_width() - 40.0).max(0.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for tab in &tabs {
                                    let index = tab.settings.selected_server;
                                    let saved = index < self.current.settings.servers.len();
                                    let active = tab.tab_id == self.current.tab_id;
                                    let color = if tab.connected {
                                        palette.green
                                    } else if tab.connecting {
                                        palette.orange
                                    } else if tab.is_error {
                                        palette.red
                                    } else {
                                        palette.muted
                                    };
                                    let status = if tab.connected {
                                        "Подключено"
                                    } else if tab.connecting {
                                        "Подключение…"
                                    } else if tab.is_error {
                                        "Ошибка · см. консоль"
                                    } else {
                                        "Отключено"
                                    };
                                    let text = if saved || tab.connected {
                                        tab.settings.server_name.as_str()
                                    } else {
                                        "Без сервера"
                                    };
                                    ui.push_id(tab.tab_id, |ui| {
                                        let frame = egui::Frame::new()
                                            .fill(if active {
                                                palette.selection
                                            } else {
                                                palette.button
                                            })
                                            .corner_radius(5)
                                            .inner_margin(egui::Margin::symmetric(6, 2));
                                        let tab_response = frame
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    let (dot, _) = ui.allocate_exact_size(
                                                        egui::vec2(7.0, 20.0),
                                                        egui::Sense::hover(),
                                                    );
                                                    ui.painter().circle_filled(
                                                        dot.center(),
                                                        3.0,
                                                        color,
                                                    );
                                                    let label = ui
                                                        .scope(|ui| {
                                                            ui.set_max_width(180.0);
                                                            ui.add(
                                                                egui::Button::new(text)
                                                                    .frame(false)
                                                                    .truncate(),
                                                            )
                                                        })
                                                        .inner
                                                        .on_hover_text(format!(
                                                            "{text}\n{} · {status}",
                                                            tab.settings.address
                                                        ));
                                                    if label.clicked() && !active {
                                                        action = Some(TabAction::Open(index));
                                                    }
                                                    if saved
                                                        && ui
                                                            .small_button("×")
                                                            .on_hover_text("Закрыть вкладку")
                                                            .clicked()
                                                    {
                                                        action = Some(TabAction::Close(index));
                                                    }
                                                });
                                            })
                                            .response;
                                        if active && reveal {
                                            tab_response.scroll_to_me(None);
                                        }
                                    });
                                }
                            });
                        });
                    ui.menu_button("+", |ui| {
                        if ui.button("Новый сервер…").clicked() {
                            action = Some(TabAction::New);
                            ui.close_menu();
                        }
                        if !self.current.settings.servers.is_empty() {
                            ui.separator();
                        }
                        for (index, profile) in self.current.settings.servers.iter().enumerate() {
                            if ui
                                .button(&profile.name)
                                .on_hover_text(&profile.address)
                                .clicked()
                            {
                                action = Some(TabAction::Open(index));
                                ui.close_menu();
                            }
                        }
                    })
                    .response
                    .on_hover_text("Открыть вкладку сервера");
                });
            });
        action
    }
}

impl eframe::App for TabbedApp {
    fn auto_save_interval(&self) -> std::time::Duration {
        self.current.auto_save_interval()
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.sync_profiles();
        self.remember_open_tabs();
        self.current.save(storage);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_os = "macos")]
        for source in crate::macos_app::take_open_requests() {
            match source {
                Ok(source) => self.current.queue_additions([source]),
                Err(error) => {
                    self.current.notice = error;
                    self.current.is_error = true;
                }
            }
        }
        for tab in &mut self.background {
            tab.receive();
        }
        self.sync_profiles();
        if crate::theme::current(ctx) != self.current.settings.theme {
            self.current.settings.theme.apply(ctx);
        }
        self.current.main_menu(ctx);
        self.current.show_navigation(ctx);
        let action = self.show_tabs(ctx);
        self.current.show_content(ctx);
        match self.current.tab_action.take().or(action) {
            Some(TabAction::Open(index)) => self.open(ctx, index),
            Some(TabAction::New) => self.new_profile(ctx),
            Some(TabAction::Close(index)) => self.close(ctx, index),
            Some(TabAction::Delete) => self.delete_profile(ctx),
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::Command;
    use std::time::{Duration, Instant};

    #[test]
    fn each_profile_restores_its_own_sort_details_tab_and_torrent() {
        let ctx = egui::Context::default();
        let mut settings = Settings {
            auto_connect: false,
            ..Default::default()
        };
        settings.save_server();
        settings.servers.push(settings.profile());
        let (tx, rx) = api::worker(ctx.clone());
        let mut app = TabbedApp {
            current: ClientApp::session(settings, String::new(), tx, rx, 0, false),
            background: Vec::new(),
            next_tab_id: 1,
            reveal_active: false,
        };
        app.current.sort = 1;
        app.current.descending = true;
        app.current.remote.restore_details_tab(4);
        app.current.selected = Some("first".into());

        let second_settings = Settings::default();
        let mut second = app.make_session(&ctx, 1, second_settings);
        second.sort = 2;
        second.remote.restore_details_tab(1);
        second.selected = Some("deleted".into());
        app.background.push(second);
        app.sync_profiles();
        app.remember_open_tabs();

        let encoded = ron::to_string(&app.current.settings).unwrap();
        let mut restored: Settings = ron::from_str(&encoded).unwrap();
        restored.select_server(0);
        let (sort, descending) = crate::columns::restored_sort(&restored);
        let (tx, rx) = api::worker(ctx.clone());
        app.current = ClientApp::session(restored, String::new(), tx, rx, sort, descending);
        app.background.clear();
        app.restore_background(&ctx);
        for (session, expected_sort, expected_tab, expected_hash) in [
            (&mut app.current, (1, true), 4, Some("first")),
            (&mut app.background[0], (2, false), 1, None),
        ] {
            assert_eq!((session.sort, session.descending), expected_sort);
            assert_eq!(session.remote.details_tab(), expected_tab);
            assert!(session.selected.is_none());
            let (tx, rx) = std::sync::mpsc::channel();
            session.rx = rx;
            tx.send((
                session.connection_epoch,
                super::super::Event::Torrents(vec![super::super::Torrent {
                    hash: "first".into(),
                    ..Default::default()
                }]),
            ))
            .unwrap();
            session.receive();
            assert_eq!(session.selected.as_deref(), expected_hash);
            assert_eq!(
                session.selection.hashes.len(),
                usize::from(expected_hash.is_some())
            );
        }
    }

    #[test]
    fn restored_tabs_keep_order_and_active_profile_and_closed_tabs_stay_closed() {
        let ctx = egui::Context::default();
        let mut settings = Settings {
            auto_connect: false,
            ..Default::default()
        };
        for name in ["Первый", "Закрытый", "Активный"] {
            settings.selected_server = settings.servers.len();
            settings.server_name = name.into();
            settings.save_server();
        }
        assert_eq!(settings.startup_server_tabs(), vec![2]); // Legacy settings.
        settings.open_server_tabs = Some(vec![0, 2, 0, 7]);
        let encoded = ron::to_string(&settings).unwrap();
        let restored: Settings = ron::from_str(&encoded).unwrap();
        let (tx, rx) = api::worker(ctx.clone());
        let mut app = TabbedApp {
            current: ClientApp::session(restored, String::new(), tx, rx, 0, false),
            background: Vec::new(),
            next_tab_id: 1,
            reveal_active: false,
        };
        app.restore_background(&ctx);
        assert_eq!(app.current.settings.selected_server, 2);
        assert_eq!(app.current.tab_id, 1);
        assert_eq!(app.background.len(), 1);
        assert_eq!(app.background[0].settings.server_name, "Первый");
        assert!(!app.background[0].connecting);
        app.remember_open_tabs();
        assert_eq!(app.current.settings.open_server_tabs, Some(vec![0, 2]));

        app.close(&ctx, 0);
        app.close(&ctx, 2);
        app.remember_open_tabs();
        assert_eq!(app.current.settings.servers.len(), 3);
        assert_eq!(app.current.settings.open_server_tabs, Some(vec![]));
        assert!(app.current.settings.startup_server_tabs().is_empty());
    }

    #[test]
    fn switching_tabs_keeps_connections_selection_and_commands_on_their_server() {
        let status = ("200 OK", r#"{"rid":1}"#);
        let old_rows = ("200 OK", r#"[{"hash":"shared","name":"Server A"}]"#);
        let new_rows = ("200 OK", r#"[{"hash":"shared","name":"Server B"}]"#);
        let (first, server_a) = api::tests::mock_server(vec![
            ("200 OK", "Ok."),
            ("200 OK", "v5.1.0"),
            old_rows,
            status,
            ("200 OK", ""),
            old_rows,
            status,
        ]);
        let (second, server_b) = api::tests::mock_server(vec![
            ("200 OK", "Ok."),
            ("200 OK", "v5.1.0"),
            new_rows,
            status,
            ("200 OK", ""),
            new_rows,
            status,
        ]);
        let ctx = egui::Context::default();
        let mut settings = Settings {
            address: first,
            remember_password: false,
            ..Default::default()
        };
        settings.save_server();
        settings.servers.push(
            Settings {
                address: second,
                remember_password: false,
                ..Default::default()
            }
            .profile(),
        );
        let (tx, rx) = api::worker(ctx.clone());
        let mut app = TabbedApp {
            current: ClientApp::session(settings, String::new(), tx, rx, 0, false),
            background: Vec::new(),
            next_tab_id: 1,
            reveal_active: false,
        };
        let wait_for_rows = |session: &mut ClientApp| {
            let deadline = Instant::now() + Duration::from_secs(4);
            while session.torrents.is_empty() {
                session.receive();
                assert!(Instant::now() < deadline, "Server did not return torrents");
                std::thread::sleep(Duration::from_millis(10));
            }
        };
        app.current.connect();
        wait_for_rows(&mut app.current);
        app.current.selected = Some("shared".into());
        app.current.selection.hashes.insert("shared".into());
        app.current.search = "Server A".into();

        app.open(&ctx, 1);
        wait_for_rows(&mut app.current);
        assert_eq!(app.current.torrents[0].name, "Server B");
        assert!(app.current.selected.is_none());
        app.current.send(Command::Stop("shared".into()));

        app.open(&ctx, 0);
        assert!(app.current.connected);
        assert_eq!(app.current.torrents[0].name, "Server A");
        assert_eq!(app.current.search, "Server A");
        assert_eq!(app.current.selected.as_deref(), Some("shared"));
        app.current
            .send(Command::Start(app.current.selected_hashes()));
        assert!(server_a.join().unwrap()[4].starts_with("POST /api/v2/torrents/start "));
        assert!(server_b.join().unwrap()[4].starts_with("POST /api/v2/torrents/stop "));

        app.delete_profile(&ctx);
        assert_eq!(app.current.settings.selected_server, 0);
        assert_eq!(app.current.settings.servers.len(), 1);
        assert_eq!(app.current.torrents[0].name, "Server B");
        assert!(app.current.connected);
        assert!(app.background.is_empty());
    }
}
