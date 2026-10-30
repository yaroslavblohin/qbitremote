use crate::model::{Settings, Torrent};
use reqwest::blocking::Client;
use std::{
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};
use url::Url;

pub enum Command {
    PrepareTorrent {
        id: u64,
        source: crate::add_torrent::Source,
    },
    CommitTorrent {
        id: u64,
        plan: Box<crate::add_torrent::Plan>,
    },
    Connect(Box<Settings>, String, u64),
    Disconnect(u64),
    Refresh,
    RssExport {
        token: u64,
        destination: std::path::PathBuf,
    },
    RssImport {
        token: u64,
        rules: serde_json::Value,
        replace: bool,
    },
    Start(String),
    Stop(String),
    Delete(String, bool),
    #[cfg(test)]
    Add(String, String),
    #[cfg(test)]
    File(std::path::PathBuf, String),
    Remote {
        token: u64,
        path: String,
        fields: Vec<(String, String)>,
        post: bool,
    },
    Export {
        token: u64,
        hash: String,
        destination: std::path::PathBuf,
    },
}
pub enum Event {
    ServerSpace(Option<u64>),
    TorrentPrepared {
        id: u64,
        result: Result<serde_json::Value, String>,
    },
    TorrentAdded {
        id: u64,
        result: Result<String, String>,
    },
    Connected(String),
    Disconnected,
    Torrents(Vec<Torrent>),
    Error(String),
    Done(String),
    Remote {
        token: u64,
        result: Result<serde_json::Value, String>,
    },
}
struct Api {
    client: Client,
    base: Url,
    modern: bool,
}

fn base_url(address: &str) -> Result<Url, String> {
    let mut url =
        Url::parse(address.trim()).map_err(|_| "Некорректный адрес сервера".to_string())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Укажите HTTP(S) адрес без логина и пароля в URL".into());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("Адрес не должен содержать параметры или #".into());
    }
    url.set_path(&format!("{}/", url.path().trim_end_matches('/')));
    Ok(url)
}

impl Api {
    fn server_space(
        &self,
        state: &mut crate::server_space::Snapshot,
    ) -> Result<Option<u64>, String> {
        let data: serde_json::Value = self
            .client
            .get(self.endpoint("sync/maindata"))
            .query(&[("rid", state.rid)])
            .header("Referer", self.base.as_str())
            .timeout(Duration::from_secs(4))
            .send()
            .map_err(error)?
            .error_for_status()
            .map_err(error)?
            .json()
            .map_err(error)?;
        state.apply(&data)
    }
    fn prepare_torrent(
        &self,
        source: &crate::add_torrent::Source,
    ) -> Result<serde_json::Value, String> {
        use crate::add_torrent::Source;
        match source {
            Source::File(path) => {
                let form = reqwest::blocking::multipart::Form::new()
                    .file("torrents", path)
                    .map_err(|e| e.to_string())?;
                let response = self
                    .client
                    .post(self.endpoint("torrents/parseMetadata"))
                    .header("Referer", self.base.as_str())
                    .multipart(form)
                    .send()
                    .map_err(error)?;
                if response.status() == reqwest::StatusCode::NOT_FOUND {
                    return Err("Сервер не поддерживает предварительный выбор файлов. Нужен qBittorrent 5.2 или новее.".into());
                }
                let values: serde_json::Value = response
                    .error_for_status()
                    .map_err(error)?
                    .json()
                    .map_err(error)?;
                values
                    .as_array()
                    .and_then(|a| a.first())
                    .cloned()
                    .ok_or("Не удалось прочитать метаданные торрента".into())
            }
            Source::Link { url, downloader } => self.remote(
                "torrents/fetchMetadata",
                &[
                    ("source".into(), url.clone()),
                    ("downloader".into(), downloader.clone()),
                ],
                true,
            ),
        }
    }
    fn commit_torrent(&self, plan: &crate::add_torrent::Plan) -> Result<String, String> {
        let fields = plan.fields()?;
        let existing = self.remote(
            "torrents/info",
            &[("hashes".into(), plan.metadata.hash.clone())],
            false,
        )?;
        if !existing
            .as_array()
            .ok_or("Сервер вернул некорректный список торрентов")?
            .is_empty()
        {
            return Err("Этот торрент уже есть на сервере. Измените его файлы и приоритеты во вкладке «Файлы».".into());
        }
        let fields = fields
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<Vec<_>>();
        add_reply(&self.post("torrents/add", &fields)?)
    }
    fn export_rss(&self, destination: &std::path::Path) -> Result<serde_json::Value, String> {
        let rules = self.remote("rss/rules", &[], false)?;
        crate::rss_rules::validate(&rules)?;
        let bytes = serde_json::to_vec_pretty(&rules).map_err(|e| e.to_string())?;
        std::fs::write(destination, bytes).map_err(|e| format!("Не удалось записать файл: {e}"))?;
        Ok(
            serde_json::json!({"summary":format!("Экспортировано правил: {} · {}",rules.as_object().unwrap().len(),destination.display())}),
        )
    }
    fn import_rss(
        &self,
        rules: &serde_json::Value,
        replace: bool,
    ) -> Result<serde_json::Value, String> {
        crate::rss_rules::validate(rules)?;
        let existing = self.remote("rss/rules", &[], false)?;
        if !existing.is_object() {
            return Err("Сервер вернул неверный список правил".into());
        }
        let mut imported = 0;
        let mut skipped = 0;
        for (name, rule) in rules.as_object().unwrap() {
            if !replace && existing.get(name).is_some() {
                skipped += 1;
                continue;
            }
            if let Err(e) = self.remote(
                "rss/setRule",
                &[
                    ("ruleName".into(), name.clone()),
                    ("ruleDef".into(), rule.to_string()),
                ],
                true,
            ) {
                return Err(format!("Импорт остановлен на «{name}»: {e}. Сохранено: {imported}, пропущено: {skipped}. Ранее применённые правила остаются на сервере."));
            }
            imported += 1;
        }
        Ok(
            serde_json::json!({"summary":format!("Импорт RSS: сохранено {imported}, пропущено {skipped}")}),
        )
    }
    fn remote(
        &self,
        path: &str,
        fields: &[(String, String)],
        post: bool,
    ) -> Result<serde_json::Value, String> {
        let request = if post {
            self.client.post(self.endpoint(path)).form(fields)
        } else {
            self.client.get(self.endpoint(path)).query(fields)
        };
        let response = request
            .header("Referer", self.base.as_str())
            .send()
            .map_err(error)?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "{path}: HTTP {}{}",
                status.as_u16(),
                if status == reqwest::StatusCode::NOT_FOUND {
                    " — функция недоступна в этой версии сервера или объект удалён"
                } else {
                    ""
                }
            ));
        }
        let body = response.text().map_err(error)?;
        if post && body.trim() == "Fails." {
            return Err(format!("{path}: сервер отклонил команду"));
        }
        if body.trim().is_empty() || body.trim() == "Ok." {
            return Ok(serde_json::Value::Null);
        }
        serde_json::from_str(&body).map_err(|_| format!("{path}: неожиданный ответ сервера"))
    }
    fn endpoint(&self, path: &str) -> String {
        self.base
            .join(&format!("api/v2/{path}"))
            .expect("static API path")
            .to_string()
    }
    fn connect(settings: Settings, password: String) -> Result<(Self, String), String> {
        let base = base_url(&settings.address)?;
        let client = Client::builder()
            .cookie_store(true)
            .timeout(Duration::from_secs(15))
            .connect_timeout(Duration::from_secs(6))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(error)?;
        let mut api = Self {
            client,
            base,
            modern: true,
        };
        let reply = api.post(
            "auth/login",
            &[
                ("username", settings.username.as_str()),
                ("password", password.as_str()),
            ],
        )?;
        if reply.trim() != "Ok." {
            return Err("Вход отклонён. Проверьте логин и пароль Web UI.".into());
        }
        let version = api
            .client
            .get(api.endpoint("app/version"))
            .send()
            .map_err(error)?
            .error_for_status()
            .map_err(error)?
            .text()
            .map_err(error)?;
        let major = version
            .trim()
            .trim_start_matches('v')
            .split('.')
            .next()
            .and_then(|s| s.parse::<u32>().ok())
            .ok_or("Сервер вернул неизвестную версию qBittorrent")?;
        if major < 4 {
            return Err("Требуется qBittorrent 4.1 или новее".into());
        }
        api.modern = major >= 5;
        Ok((api, version.trim().into()))
    }
    fn post(&self, path: &str, fields: &[(&str, &str)]) -> Result<String, String> {
        let response = self
            .client
            .post(self.endpoint(path))
            .header("Referer", self.base.as_str())
            .form(fields)
            .send()
            .map_err(error)?;
        command_reply(response)
    }
    fn torrents(&self) -> Result<Vec<Torrent>, String> {
        self.client
            .get(self.endpoint("torrents/info"))
            .send()
            .map_err(error)?
            .error_for_status()
            .map_err(error)?
            .json()
            .map_err(error)
    }
    fn execute(&self, command: Command) -> Result<String, String> {
        match command {
            Command::Start(hash) => {
                self.post(
                    if self.modern {
                        "torrents/start"
                    } else {
                        "torrents/resume"
                    },
                    &[("hashes", &hash)],
                )?;
            }
            Command::Stop(hash) => {
                self.post(
                    if self.modern {
                        "torrents/stop"
                    } else {
                        "torrents/pause"
                    },
                    &[("hashes", &hash)],
                )?;
            }
            Command::Delete(hash, files) => {
                self.post(
                    "torrents/delete",
                    &[
                        ("hashes", &hash),
                        ("deleteFiles", if files { "true" } else { "false" }),
                    ],
                )?;
            }
            #[cfg(test)]
            Command::Add(links, path) => {
                let body = self.post("torrents/add", &[("urls", &links), ("savepath", &path)])?;
                return add_reply(&body);
            }
            #[cfg(test)]
            Command::File(file, path) => {
                let form = reqwest::blocking::multipart::Form::new()
                    .text("savepath", path)
                    .file("torrents", file)
                    .map_err(|e| e.to_string())?;
                let response = self
                    .client
                    .post(self.endpoint("torrents/add"))
                    .header("Referer", self.base.as_str())
                    .multipart(form)
                    .send()
                    .map_err(error)?;
                let body = command_reply(response)?;
                return add_reply(&body);
            }
            _ => {}
        }
        Ok("Команда выполнена".into())
    }
}
// Newer qBittorrent versions return structured results from torrents/add.
// Keep this separate from login and other command response formats.
fn add_reply(body: &str) -> Result<String, String> {
    if body.trim() == "Ok." {
        return Ok("Запрос на добавление торрента принят".into());
    }
    if body.trim() == "Fails." {
        return Err("Сервер не принял торрент".into());
    }
    let result: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| "Не удалось распознать ответ добавления. Проверьте список торрентов перед повторной попыткой.".to_string())?;
    let counts = (
        result["success_count"].as_u64(),
        result["pending_count"].as_u64(),
        result["failure_count"].as_u64(),
    );
    let (Some(success), Some(pending), Some(failed)) = counts else {
        return Err("Неизвестный формат ответа добавления. Проверьте список торрентов перед повторной попыткой.".into());
    };
    if success == 0 && pending == 0 {
        return Err(format!("Сервер не добавил торренты. Отказов: {failed}"));
    }
    let mut message = format!("Добавлено: {success}");
    if pending > 0 {
        message.push_str(&format!(" · Ожидают обработки сервером: {pending}"));
    }
    if failed > 0 {
        message.push_str(&format!(" · Не добавлено: {failed}"));
    }
    Ok(message)
}

// qBittorrent can use 204 for successful commands, including login.
// Keep the legacy body check: a 200 response containing Fails. or an
// unexpected empty page must not be accepted as a successful login/add.
fn command_reply(response: reqwest::blocking::Response) -> Result<String, String> {
    let response = response.error_for_status().map_err(error)?;
    if response.status() == reqwest::StatusCode::NO_CONTENT {
        Ok("Ok.".into())
    } else {
        response.text().map_err(error)
    }
}

fn error(e: reqwest::Error) -> String {
    e.without_url().to_string()
}

pub fn worker(ctx: eframe::egui::Context) -> (Sender<Command>, Receiver<(u64, Event)>) {
    let (tx, rx) = mpsc::channel();
    let (events, out) = mpsc::channel();
    std::thread::spawn(move || {
        let epoch = std::cell::Cell::new(0_u64);
        let emit = |event| {
            let _ = events.send((epoch.get(), event));
            ctx.request_repaint();
        };
        let mut api: Option<Api> = None;
        let mut next_poll = Instant::now();
        let mut space = crate::server_space::Snapshot::default();
        loop {
            let wait = if api.is_some() {
                next_poll.saturating_duration_since(Instant::now())
            } else {
                Duration::from_secs(60)
            };
            match rx.recv_timeout(wait) {
                Ok(Command::Connect(settings, password, session)) => {
                    space = Default::default();
                    epoch.set(session);
                    api = None;
                    match Api::connect(*settings, password) {
                        Ok((connected, version)) => {
                            api = Some(connected);
                            emit(Event::Connected(version));
                            next_poll = Instant::now();
                        }
                        Err(e) => {
                            emit(Event::Disconnected);
                            emit(Event::Error(e));
                        }
                    }
                }
                Ok(Command::Disconnect(session)) => {
                    epoch.set(session);
                    api = None;
                    emit(Event::Disconnected);
                }
                Ok(Command::Refresh) => next_poll = Instant::now(),
                Ok(Command::PrepareTorrent { id, source }) => {
                    let result = api
                        .as_ref()
                        .ok_or_else(|| "Нет подключения".to_string())
                        .and_then(|a| a.prepare_torrent(&source));
                    emit(Event::TorrentPrepared { id, result });
                }
                Ok(Command::CommitTorrent { id, plan }) => {
                    let result = api
                        .as_ref()
                        .ok_or_else(|| "Нет подключения".to_string())
                        .and_then(|a| a.commit_torrent(&plan));
                    emit(Event::TorrentAdded { id, result });
                    next_poll = Instant::now();
                }
                Ok(Command::RssImport {
                    token,
                    rules,
                    replace,
                }) => {
                    let result = api
                        .as_ref()
                        .ok_or_else(|| "Нет подключения".to_string())
                        .and_then(|a| a.import_rss(&rules, replace));
                    emit(Event::Remote { token, result });
                }
                Ok(Command::RssExport { token, destination }) => {
                    let result = api
                        .as_ref()
                        .ok_or_else(|| "Нет подключения".to_string())
                        .and_then(|a| a.export_rss(&destination));
                    emit(Event::Remote { token, result });
                }
                Ok(Command::Remote {
                    token,
                    path,
                    fields,
                    post,
                }) => {
                    let result = api
                        .as_ref()
                        .ok_or_else(|| "Нет подключения".into())
                        .and_then(|api| api.remote(&path, &fields, post));
                    emit(Event::Remote { token, result });
                    if post {
                        next_poll = Instant::now();
                    }
                }
                Ok(Command::Export {
                    token,
                    hash,
                    destination,
                }) => {
                    let result = (|| {
                        let api = api.as_ref().ok_or("Нет подключения")?;
                        let response = api
                            .client
                            .get(api.endpoint("torrents/export"))
                            .query(&[("hash", hash)])
                            .send()
                            .map_err(error)?
                            .error_for_status()
                            .map_err(error)?;
                        let data = response.bytes().map_err(error)?;
                        std::fs::write(destination, data).map_err(|e| e.to_string())?;
                        Ok(serde_json::Value::Null)
                    })();
                    emit(Event::Remote { token, result });
                }
                Ok(command) => {
                    if let Some(api) = &api {
                        match api.execute(command) {
                            Ok(msg) => emit(Event::Done(msg)),
                            Err(e) => emit(Event::Error(e)),
                        }
                        next_poll = Instant::now();
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            if Instant::now() >= next_poll {
                if let Some(client) = &api {
                    match client.torrents() {
                        Ok(torrents) => {
                            emit(Event::Torrents(torrents));
                            let free = match client.server_space(&mut space) {
                                Ok(free) => free,
                                Err(_) => {
                                    space = Default::default();
                                    None
                                }
                            };
                            emit(Event::ServerSpace(free));
                        }
                        Err(e) => {
                            emit(Event::Error(format!(
                                "Обновление не удалось: {e}. Подключитесь повторно."
                            )));
                            api = None;
                            emit(Event::Disconnected);
                        }
                    }
                }
                next_poll = Instant::now() + Duration::from_secs(3);
            }
        }
    });
    (tx, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_or_all_skipped_plan_sends_no_server_mutations() {
        for priority in [0, 2] {
            let (address, server) = mock_server(vec![("204 No Content", ""), ("200 OK", "v5.2.3")]);
            let (api, _) = Api::connect(
                Settings {
                    address,
                    ..Default::default()
                },
                "test".into(),
            )
            .unwrap();
            let plan = crate::add_torrent::Plan {
                metadata: crate::add_torrent::Metadata {
                    hash: "a".repeat(40),
                    name: "test".into(),
                    files: vec![crate::add_torrent::File {
                        path: "one".into(),
                        size: 1,
                        priority,
                    }],
                },
                path: String::new(),
                category: String::new(),
                tags: String::new(),
                start: true,
                top: false,
                sequential: false,
            };
            assert!(api.commit_torrent(&plan).is_err());
            assert_eq!(server.join().unwrap().len(), 2);
        }
    }
    #[test]
    #[ignore = "requires explicit server credentials; reads server status only"]
    fn live_server_space() {
        let (api, _) = Api::connect(
            Settings {
                address: std::env::var("QBIT_TEST_ADDRESS").unwrap(),
                username: std::env::var("QBIT_TEST_USERNAME").unwrap(),
                ..Default::default()
            },
            std::env::var("QBIT_TEST_PASSWORD").unwrap(),
        )
        .unwrap();
        let mut state = crate::server_space::Snapshot::default();
        let free = api.server_space(&mut state).unwrap();
        assert!(state.rid > 0);
        let _ = api.server_space(&mut state).unwrap();
        println!(
            "Server free space: {}",
            free.map(crate::model::bytes)
                .unwrap_or_else(|| "unavailable".into())
        );
    }
    #[test]
    fn server_space_polls_incrementally_and_reports_http_errors() {
        let (address, server) = mock_server(vec![
            ("204 No Content", ""),
            ("200 OK", "v5.2.3"),
            (
                "200 OK",
                r#"{"rid":42,"full_update":true,"server_state":{"free_space_on_disk":987654321000}}"#,
            ),
            ("200 OK", r#"{"rid":43,"server_state":{"dl_info_speed":5}}"#),
            ("403 Forbidden", ""),
        ]);
        let (api, _) = Api::connect(
            Settings {
                address,
                ..Default::default()
            },
            "test".into(),
        )
        .unwrap();
        let mut state = crate::server_space::Snapshot::default();
        assert_eq!(api.server_space(&mut state).unwrap(), Some(987654321000));
        assert_eq!(api.server_space(&mut state).unwrap(), Some(987654321000));
        assert!(api.server_space(&mut state).is_err());
        let requests = server.join().unwrap();
        assert!(requests[2].starts_with("GET /api/v2/sync/maindata?rid=0 "));
        assert!(requests[3].starts_with("GET /api/v2/sync/maindata?rid=42 "));
        assert!(requests[4].starts_with("GET /api/v2/sync/maindata?rid=43 "));
    }
    #[test]
    fn selected_files_are_applied_in_the_add_request_without_early_start() {
        for start in [false, true] {
            let (address, server) = mock_server(vec![
                ("204 No Content", ""),
                ("200 OK", "v5.2.3"),
                ("200 OK", "[]"),
                ("204 No Content", ""),
            ]);
            let (api, _) = Api::connect(
                Settings {
                    address,
                    ..Default::default()
                },
                "test".into(),
            )
            .unwrap();
            let plan = crate::add_torrent::Plan {
                metadata: crate::add_torrent::Metadata {
                    hash: "a".repeat(40),
                    name: "test".into(),
                    files: vec![
                        crate::add_torrent::File {
                            path: "one".into(),
                            size: 10,
                            priority: 0,
                        },
                        crate::add_torrent::File {
                            path: "two".into(),
                            size: 20,
                            priority: 6,
                        },
                    ],
                },
                path: "/data".into(),
                category: "TV".into(),
                tags: String::new(),
                start,
                top: false,
                sequential: false,
            };
            api.commit_torrent(&plan).unwrap();
            let requests = server.join().unwrap();
            assert!(requests[2].starts_with("GET /api/v2/torrents/info?"));
            assert!(requests[3].starts_with("POST /api/v2/torrents/add "));
            let fields: std::collections::HashMap<_, _> = url::form_urlencoded::parse(
                requests[3].split("\r\n\r\n").nth(1).unwrap().as_bytes(),
            )
            .into_owned()
            .collect();
            assert_eq!(fields["filePriorities"], "0,6");
            assert_eq!(fields["stopped"], (!start).to_string());
            assert_eq!(fields["category"], "TV");
            assert_eq!(requests.len(), 4);
        }
    }
    #[test]
    fn existing_torrent_is_never_changed_by_new_add_dialog() {
        let (address, server) = mock_server(vec![
            ("204 No Content", ""),
            ("200 OK", "v5.2.3"),
            (
                "200 OK",
                r#"[{"hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}]"#,
            ),
        ]);
        let (api, _) = Api::connect(
            Settings {
                address,
                ..Default::default()
            },
            "test".into(),
        )
        .unwrap();
        let plan = crate::add_torrent::Plan {
            metadata: crate::add_torrent::Metadata {
                hash: "a".repeat(40),
                name: "test".into(),
                files: vec![crate::add_torrent::File {
                    path: "one".into(),
                    size: 10,
                    priority: 1,
                }],
            },
            path: String::new(),
            category: String::new(),
            tags: String::new(),
            start: true,
            top: false,
            sequential: false,
        };
        assert!(api.commit_torrent(&plan).unwrap_err().contains("уже есть"));
        assert_eq!(server.join().unwrap().len(), 3);
    }
    #[test]
    #[ignore = "requires explicit server credentials; metadata only, no torrent is added"]
    fn live_metadata_preview_only() {
        let (api, _) = Api::connect(
            Settings {
                address: std::env::var("QBIT_TEST_ADDRESS").unwrap(),
                username: std::env::var("QBIT_TEST_USERNAME").unwrap(),
                ..Default::default()
            },
            std::env::var("QBIT_TEST_PASSWORD").unwrap(),
        )
        .unwrap();
        let before = api.torrents().unwrap().len();
        let path =
            std::env::temp_dir().join(format!("qbit-preview-{}.torrent", std::process::id()));
        std::fs::write(&path,b"d4:infod6:lengthi1e4:name4:test12:piece lengthi16384e6:pieces20:12345678901234567890ee").unwrap();
        let result = api.prepare_torrent(&crate::add_torrent::Source::File(path.clone()));
        std::fs::remove_file(path).unwrap();
        let meta = crate::add_torrent::Metadata::parse(&result.unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(meta.files.len(), 1);
        assert_eq!(api.torrents().unwrap().len(), before);
        println!(
            "Metadata parsed; {} file; no torrent added",
            meta.files.len()
        );
    }
    #[test]
    fn shell_torrent_file_is_uploaded_as_multipart() {
        let (address, server) = mock_server(vec![
            ("204 No Content", ""),
            ("200 OK", "v5.2.3"),
            ("204 No Content", ""),
        ]);
        let (api, _) = Api::connect(
            Settings {
                address,
                ..Default::default()
            },
            "test".into(),
        )
        .unwrap();
        let path = std::env::temp_dir().join(format!("qbit shell {}.torrent", std::process::id()));
        let payload =
            b"d4:infod4:name4:test6:lengthi1e12:piece lengthi1e6:pieces20:12345678901234567890ee";
        std::fs::write(&path, payload).unwrap();
        api.execute(Command::File(path.clone(), String::new()))
            .unwrap();
        let requests = server.join().unwrap();
        assert!(requests[2].starts_with("POST /api/v2/torrents/add "));
        assert!(requests[2].contains("name=\"torrents\"; filename="));
        assert!(requests[2].contains(std::str::from_utf8(payload).unwrap()));
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn rss_export_roundtrip_and_http_failure_preserve_destination() {
        let (address, server) = mock_server(vec![
            ("204 No Content", ""),
            ("200 OK", "v5.2.3"),
            (
                "200 OK",
                r#"{"Example":{"enabled":true,"affectedFeeds":[]}}"#,
            ),
            ("403 Forbidden", ""),
        ]);
        let (api, _) = Api::connect(
            Settings {
                address,
                ..Default::default()
            },
            "test".into(),
        )
        .unwrap();
        let path = std::env::temp_dir().join(format!(
            "qbit-rss-export-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        api.export_rss(&path).unwrap();
        let original = std::fs::read(&path).unwrap();
        assert_eq!(
            crate::rss_rules::parse(&original).unwrap()["Example"]["enabled"],
            true
        );
        assert!(api.export_rss(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::remove_file(path).unwrap();
        server.join().unwrap();
    }
    #[test]
    fn rss_import_skips_conflicts_and_preserves_rule_fields() {
        let (address, server) = mock_server(vec![
            ("204 No Content", ""),
            ("200 OK", "v5.2.3"),
            ("200 OK", r#"{"Existing":{"enabled":true}}"#),
            ("204 No Content", ""),
        ]);
        let (api, _) = Api::connect(
            Settings {
                address,
                ..Default::default()
            },
            "test".into(),
        )
        .unwrap();
        let rules = serde_json::json!({"Existing":{"enabled":false},"Новое & правило":{"enabled":true,"torrentParams":{"save_path":"/downloads/TV"},"futureField":123}});
        let reply = api.import_rss(&rules, false).unwrap();
        assert!(reply["summary"]
            .as_str()
            .unwrap()
            .contains("сохранено 1, пропущено 1"));
        let requests = server.join().unwrap();
        assert!(requests[3].starts_with("POST /api/v2/rss/setRule "));
        let body = requests[3].split("\r\n\r\n").nth(1).unwrap();
        let fields: std::collections::HashMap<_, _> = url::form_urlencoded::parse(body.as_bytes())
            .into_owned()
            .collect();
        assert_eq!(fields["ruleName"], "Новое & правило");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&fields["ruleDef"]).unwrap(),
            rules["Новое & правило"]
        );
    }
    #[test]
    fn rss_import_reports_partial_failure_and_replaces_only_when_requested() {
        let (address, server) = mock_server(vec![
            ("204 No Content", ""),
            ("200 OK", "v5.2.3"),
            ("200 OK", r#"{"A":{"enabled":true}}"#),
            ("204 No Content", ""),
            ("403 Forbidden", ""),
        ]);
        let (api, _) = Api::connect(
            Settings {
                address,
                ..Default::default()
            },
            "test".into(),
        )
        .unwrap();
        let result=api.import_rss(&serde_json::json!({"A":{"enabled":false},"B":{"enabled":true},"C":{"enabled":true}}),true).unwrap_err();
        assert!(result.contains("Сохранено: 1"));
        assert!(result.contains("«B»"));
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 5);
        assert!(requests[3].contains("ruleName=A"));
    }
    #[test]
    fn multipart_add_accepts_structured_legacy_and_pending_replies() {
        let file = std::env::temp_dir().join(format!(
            "qbit-add-test-{}-{}.torrent",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&file, b"d4:infod4:name4:testee").unwrap();
        for (status, body, accepted) in [
            (
                "200 OK",
                r#"{"success_count":1,"pending_count":0,"failure_count":0,"added_torrent_ids":["abc"]}"#,
                true,
            ),
            (
                "202 Accepted",
                r#"{"success_count":0,"pending_count":1,"failure_count":0,"added_torrent_ids":[]}"#,
                true,
            ),
            ("200 OK", "Ok.", true),
            ("204 No Content", "", true),
            ("200 OK", "Fails.", false),
            ("409 Conflict", "", false),
            ("200 OK", "", false),
            (
                "200 OK",
                r#"{"success_count":0,"pending_count":0,"failure_count":1}"#,
                false,
            ),
        ] {
            let (address, server) = mock_server(vec![
                ("200 OK", "Ok."),
                ("200 OK", "v5.2.3"),
                (status, body),
            ]);
            let (api, _) = Api::connect(
                Settings {
                    address,
                    ..Default::default()
                },
                "test".into(),
            )
            .unwrap();
            let result = api.execute(Command::File(file.clone(), "/downloads".into()));
            let requests = server.join().unwrap();
            assert_eq!(result.is_ok(), accepted, "{status}: {body}: {result:?}");
            assert!(requests[2].starts_with("POST /api/v2/torrents/add "));
            assert!(requests[2].contains("name=\"torrents\""));
            assert!(requests[2].contains("d4:infod4:name4:testee"));
            assert!(requests[2].contains("/downloads"));
        }
        std::fs::remove_file(file).unwrap();
    }

    #[test]
    fn add_reports_partial_results_and_rejects_unknown_payloads() {
        let message =
            add_reply(r#"{"success_count":2,"pending_count":1,"failure_count":3}"#).unwrap();
        assert!(message.contains("Добавлено: 2"));
        assert!(message.contains("сервером: 1"));
        assert!(message.contains("Не добавлено: 3"));
        for body in [
            "{}",
            "[]",
            "<html>Login</html>",
            r#"{"success_count":-1,"pending_count":0,"failure_count":0}"#,
        ] {
            assert!(add_reply(body).is_err());
        }
        let (address, server) = mock_server(vec![
            ("200 OK", "Ok."),
            ("200 OK", "v5.2.3"),
            (
                "200 OK",
                r#"{"success_count":1,"pending_count":0,"failure_count":0}"#,
            ),
        ]);
        let (api, _) = Api::connect(
            Settings {
                address,
                ..Default::default()
            },
            "test".into(),
        )
        .unwrap();
        assert!(api
            .execute(Command::Add(
                "magnet:?xt=urn:btih:abc".into(),
                String::new()
            ))
            .is_ok());
        server.join().unwrap();
    }
    #[test]
    fn remote_requests_encode_limits_and_preserve_session() {
        let (address, server) = mock_server(vec![
            ("204 No Content", ""),
            ("200 OK", "v5.2.3"),
            ("200 OK", r#"{"dl_rate_limit":1536}"#),
            ("204 No Content", ""),
            ("200 OK", ""),
            ("404 Not Found", ""),
            ("200 OK", "Fails."),
        ]);
        let (api, _) = Api::connect(
            Settings {
                address,
                ..Default::default()
            },
            "test".into(),
        )
        .unwrap();
        assert_eq!(
            api.remote("transfer/info", &[], false).unwrap()["dl_rate_limit"],
            1536
        );
        api.remote(
            "transfer/setDownloadLimit",
            &[("limit".into(), "1536".into())],
            true,
        )
        .unwrap();
        api.remote(
            "torrents/setUploadLimit",
            &[
                ("hashes".into(), "a|b".into()),
                ("limit".into(), "0".into()),
            ],
            true,
        )
        .unwrap();
        assert!(api
            .remote("torrents/addWebSeeds", &[], true)
            .unwrap_err()
            .contains("404"));
        assert!(api.remote("torrents/add", &[], true).is_err());
        let requests = server.join().unwrap();
        assert!(requests[2].starts_with("GET /api/v2/transfer/info "));
        assert!(requests[3].starts_with("POST /api/v2/transfer/setDownloadLimit "));
        assert!(requests[3].ends_with("limit=1536"));
        assert!(requests[4].ends_with("hashes=a%7Cb&limit=0"));
        assert!(requests[4]
            .to_lowercase()
            .contains("cookie: sid=test-session"));
    }
    #[test]
    fn login_204_keeps_session_and_reads_torrents() {
        let (address, server) = mock_server(vec![
            ("204 No Content", ""),
            ("200 OK", "v5.2.3"),
            ("200 OK", "[]"),
            ("204 No Content", ""),
        ]);
        let (api, version) = Api::connect(
            Settings {
                address,
                ..Default::default()
            },
            "test".into(),
        )
        .unwrap();
        assert_eq!(version, "v5.2.3");
        assert!(api.torrents().unwrap().is_empty());
        api.execute(Command::Add(
            "https://example.org/test.torrent".into(),
            String::new(),
        ))
        .unwrap();
        let requests = server.join().unwrap();
        assert!(requests[2]
            .to_lowercase()
            .contains("cookie: sid=test-session"));
    }

    #[test]
    fn empty_200_login_is_rejected() {
        let (address, server) = mock_server(vec![("200 OK", "")]);
        assert!(Api::connect(
            Settings {
                address,
                ..Default::default()
            },
            "test".into()
        )
        .is_err());
        server.join().unwrap();
    }

    #[test]
    #[ignore = "requires explicit server credentials in QBIT_TEST_* environment variables"]
    fn live_server_read_only() {
        let settings = Settings {
            address: std::env::var("QBIT_TEST_ADDRESS").expect("QBIT_TEST_ADDRESS"),
            username: std::env::var("QBIT_TEST_USERNAME").expect("QBIT_TEST_USERNAME"),
            ..Default::default()
        };
        let password = std::env::var("QBIT_TEST_PASSWORD").expect("QBIT_TEST_PASSWORD");
        let (api, version) = Api::connect(settings, password).unwrap();
        let torrents = api.torrents().unwrap();
        println!(
            "Connected to {version}; decoded {} torrents",
            torrents.len()
        );
        for endpoint in [
            "transfer/info",
            "transfer/speedLimitsMode",
            "app/preferences",
            "app/buildInfo",
            "torrents/categories",
            "torrents/tags",
            "rss/items",
            "rss/rules",
            "log/main",
            "log/peers",
            "search/plugins",
            "search/status",
        ] {
            api.remote(endpoint, &[], false)
                .unwrap_or_else(|error| panic!("{endpoint}: {error}"));
            println!("Read-only endpoint OK: {endpoint}");
        }
        if let Some(torrent) = torrents.first() {
            for endpoint in [
                "torrents/properties",
                "torrents/trackers",
                "torrents/files",
                "torrents/webseeds",
                "torrents/pieceStates",
                "sync/torrentPeers",
            ] {
                api.remote(endpoint, &[("hash".into(), torrent.hash.clone())], false)
                    .unwrap_or_else(|error| panic!("{endpoint}: {error}"));
                println!("Read-only torrent endpoint OK: {endpoint}");
            }
        }
    }
    #[test]
    fn reverse_proxy_path_preserved() {
        let base = base_url("https://example.org/qbit").unwrap();
        assert_eq!(
            base.join("api/v2/torrents/info").unwrap().as_str(),
            "https://example.org/qbit/api/v2/torrents/info"
        );
    }
    #[test]
    fn rejects_ambiguous_addresses() {
        for value in [
            "ftp://host",
            "http://admin:secret@host",
            "http://host?x=1",
            "garbage",
        ] {
            assert!(base_url(value).is_err());
        }
    }

    fn mock_server(
        replies: Vec<(&'static str, &'static str)>,
    ) -> (String, std::thread::JoinHandle<Vec<String>>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let task = std::thread::spawn(move || {
            let mut requests = vec![];
            for (status, body) in replies {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut data = Vec::new();
                loop {
                    let mut buf = [0; 4096];
                    let n = stream.read(&mut buf).unwrap();
                    assert!(n > 0, "request ended prematurely");
                    data.extend_from_slice(&buf[..n]);
                    if let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&data[..end]).to_lowercase();
                        let length: usize = headers
                            .lines()
                            .find_map(|l| l.strip_prefix("content-length:"))
                            .map(|s| s.trim().parse().unwrap())
                            .unwrap_or(0);
                        if data.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                requests.push(String::from_utf8(data).unwrap());
                write!(stream, "HTTP/1.1 {status}\r\nContent-Length: {}\r\nSet-Cookie: SID=test-session; Path=/\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
            requests
        });
        (address, task)
    }

    #[test]
    fn authenticated_commands_support_v4_and_v5() {
        for (version, start, stop) in [("v4.6.7", "resume", "pause"), ("v5.1.0", "start", "stop")] {
            let (address, server) = mock_server(vec![
                ("200 OK", "Ok."),
                ("200 OK", version),
                ("200 OK", "[]"),
                ("200 OK", ""),
                ("200 OK", ""),
                ("200 OK", ""),
            ]);
            let (api, _) = Api::connect(
                Settings {
                    address,
                    ..Default::default()
                },
                "a&b".into(),
            )
            .unwrap();
            assert!(api.torrents().unwrap().is_empty());
            api.execute(Command::Start("abc|def".into())).unwrap();
            api.execute(Command::Stop("abc|def".into())).unwrap();
            api.execute(Command::Delete("abc|def".into(), false))
                .unwrap();
            let requests = server.join().unwrap();
            assert!(requests[0].contains("password=a%26b"));
            assert!(requests[1]
                .to_lowercase()
                .contains("cookie: sid=test-session"));
            assert!(requests[3].starts_with(&format!("POST /api/v2/torrents/{start} ")));
            assert!(requests[4].starts_with(&format!("POST /api/v2/torrents/{stop} ")));
            assert!(requests[5].contains("deleteFiles=false"));
            for request in &requests[3..=5] {
                assert!(request.contains("hashes=abc%7Cdef"));
            }
        }
    }
    #[test]
    fn failed_login_is_not_accepted() {
        let (address, server) = mock_server(vec![("200 OK", "Fails.")]);
        assert!(Api::connect(
            Settings {
                address,
                ..Default::default()
            },
            "bad".into()
        )
        .is_err());
        server.join().unwrap();
    }
    #[test]
    fn http_error_propagates() {
        let (address, server) = mock_server(vec![("403 Forbidden", "Forbidden")]);
        assert!(Api::connect(
            Settings {
                address,
                ..Default::default()
            },
            "bad".into()
        )
        .is_err());
        server.join().unwrap();
    }
}
