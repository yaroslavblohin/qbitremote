use super::*;

impl Remote {
    fn rss_transfer(&mut self, export: Option<std::path::PathBuf>, rules: Value, replace: bool) {
        self.serial += 1;
        let title = if export.is_some() {
            "Экспорт RSS"
        } else {
            "Импорт RSS"
        };
        self.pending.insert(
            self.serial,
            Pending {
                key: title.into(),
                post: true,
            },
        );
        self.queue.push(if let Some(destination) = export {
            Command::RssExport {
                token: self.serial,
                destination,
            }
        } else {
            Command::RssImport {
                token: self.serial,
                rules,
                replace,
            }
        });
        self.message = format!("{title}: выполнение…");
        self.error = false;
    }
    pub(super) fn rss_import_dialog(&mut self, ctx: &egui::Context, connected: bool) {
        let Some((rules, mut replace)) = self.rss_import.take() else {
            return;
        };
        let mut open = true;
        let mut submit = false;
        let mut cancel = false;
        egui::Window::new("Импорт правил RSS").open(&mut open).collapsible(false).default_width(520.0).show(ctx,|ui|{
            let map=rules.as_object().unwrap();
            ui.label(format!("Правил в файле: {}",map.len()));
            ui.label("Правила будут добавлены на подключённый сервер. URL лент и пути сохраняются как в файле; сами ленты не создаются. Включённые правила могут начать загрузку сразу.");
            ui.checkbox(&mut replace,"Заменять существующие правила с такими же именами");
            ui.weak("По умолчанию совпадения пропускаются. Перед импортом список правил повторно читается с сервера.");
            egui::ScrollArea::vertical().max_height(230.0).show(ui,|ui|{
                let existing=self.cache.get("rss-rules").and_then(|c|c.data.as_ref().ok());
                for name in map.keys(){let collision=existing.is_some_and(|v|v.get(name).is_some());ui.label(format!("{name}{}",if collision {if replace{" — заменить"}else{" — пропустить"}}else{""}));}
            });
            ui.horizontal(|ui|{submit=ui.add_enabled(connected&&!map.is_empty(),egui::Button::new("Импортировать")).clicked();cancel=ui.button("Отмена").clicked();});
        });
        if submit {
            self.rss_transfer(None, rules, replace);
        } else if open && !cancel {
            self.rss_import = Some((rules, replace));
        }
    }
    pub fn page_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading(PAGES.iter().find(|(p, _)| *p == self.page).unwrap().1);
        ui.separator();
        match self.page {
            Page::Speed => self.speed_page(ui),
            Page::Search => self.search_page(ui),
            Page::Rss => self.rss_page(ui),
            Page::Log => self.log_page(ui),
            Page::Preferences => self.preferences_page(ui),
            Page::Categories => self.categories_page(ui),
            Page::Torrents => {}
        }
    }
    fn speed_page(&mut self, ui: &mut egui::Ui) {
        let info = self.get("transfer", "transfer/info", vec![]);
        let mode = self.get("speed-mode", "transfer/speedLimitsMode", vec![]);
        if let Some(info) = info {
            ui.horizontal_wrapped(|ui| {
                self.limit_button(ui, None, true, number(&info, "dl_rate_limit"));
                self.limit_button(ui, None, false, number(&info, "up_rate_limit"));
                if let Some(mode) = mode {
                    let mut alternate = mode.as_i64() == Some(1);
                    if ui
                        .checkbox(&mut alternate, "Альтернативные лимиты")
                        .changed()
                    {
                        self.action(
                            "Переключить режим скорости",
                            "transfer/toggleSpeedLimitsMode",
                            vec![],
                        );
                    }
                }
            });
            egui::ScrollArea::vertical().show(ui, |ui| {
                speed_graph(ui, &self.history);
                object_grid(ui, "transfer-info", &info);
                ui.label("Альтернативные лимиты и расписание доступны в настройках сервера.");
                if let Some(build) = self.get("build", "app/buildInfo", vec![]) {
                    ui.collapsing("Сборка сервера", |ui| {
                        object_grid(ui, "build", &build)
                    });
                }
            });
        } else {
            self.loading(ui, "transfer");
        }
    }
    fn search_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search_pattern)
                    .hint_text("Что найти?")
                    .desired_width(260.0),
            );
            if ui
                .add_enabled(
                    !self.search_pattern.trim().is_empty(),
                    egui::Button::new("Найти"),
                )
                .clicked()
            {
                self.action(
                    "Начать поиск",
                    "search/start",
                    vec![
                        ("pattern".into(), self.search_pattern.clone()),
                        ("category".into(), self.search_category.clone()),
                        ("plugins".into(), self.search_plugins.clone()),
                    ],
                );
            }
            if let Some(id) = self.search_id {
                if ui.button("Остановить").clicked() {
                    self.action(
                        "Остановить поиск",
                        "search/stop",
                        vec![("id".into(), id.to_string())],
                    );
                }
                if ui.button("Удалить поиск").clicked() {
                    self.action(
                        "Удалить поиск",
                        "search/delete",
                        vec![("id".into(), id.to_string())],
                    );
                    self.search_id = None;
                }
            }
        });
        ui.collapsing("Плагины и параметры поиска",|ui|{
            ui.horizontal(|ui|{ui.label("Категория");ui.text_edit_singleline(&mut self.search_category);ui.label("Плагины");ui.text_edit_singleline(&mut self.search_plugins);});
            ui.weak("all — все категории; enabled — включённые плагины; имена нескольких плагинов через |");
            if ui.button("Установить плагин по URL…").clicked(){self.open_form("Установить плагин","search/installPlugin",vec![],vec![Field::new("sources","URL плагина","")],"Код плагина будет выполняться на сервере. Используйте доверенный источник.");}
            if ui.button("Обновить плагины").clicked(){self.action("Обновить плагины","search/updatePlugins",vec![]);}
            if let Some(plugins)=self.get("plugins","search/plugins",vec![]){for p in plugins.as_array().into_iter().flatten(){ui.horizontal_wrapped(|ui|{let name=text(p,"name");let mut enabled=p["enabled"].as_bool().unwrap_or(false);if ui.checkbox(&mut enabled,format!("{} · {}",text(p,"fullName"),text(p,"version"))).changed(){self.action("Переключить плагин","search/enablePlugin",vec![("names".into(),name.clone()),("enable".into(),enabled.to_string())]);}ui.label(format!("{name} · {}",p["supportedCategories"]));if ui.small_button("Удалить").clicked(){self.open_form("Удалить плагин","search/uninstallPlugin",vec![("names".into(),name)],vec![],"Удалить плагин поиска с сервера?");}});}}else{self.loading(ui,"plugins");}
        });
        if let Some(status) = self.get("search-status", "search/status", vec![]) {
            ui.horizontal_wrapped(|ui| {
                for s in status.as_array().into_iter().flatten() {
                    if ui
                        .selectable_label(
                            self.search_id == s["id"].as_i64(),
                            format!(
                                "{} · {} · {}",
                                text(s, "pattern"),
                                text(s, "status"),
                                number(s, "total")
                            ),
                        )
                        .clicked()
                    {
                        self.search_id = s["id"].as_i64();
                        self.search_offset = 0;
                    }
                }
            });
        }
        let Some(id) = self.search_id else { return };
        let key = format!("search/{id}/{}", self.search_offset);
        let Some(result) = self.get(
            &key,
            "search/results",
            vec![
                ("id".into(), id.to_string()),
                ("offset".into(), self.search_offset.to_string()),
                ("limit".into(), "100".into()),
            ],
        ) else {
            self.loading(ui, &key);
            return;
        };
        let rows = result["results"].as_array().cloned().unwrap_or_default();
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.search_offset > 0, egui::Button::new("Назад"))
                .clicked()
            {
                self.search_offset = self.search_offset.saturating_sub(100);
            }
            ui.label(format!(
                "{}–{}",
                self.search_offset + 1,
                self.search_offset + rows.len()
            ));
            if ui
                .add_enabled(rows.len() == 100, egui::Button::new("Далее"))
                .clicked()
            {
                self.search_offset += 100;
            }
        });
        data_table(
            ui,
            "search-results",
            &rows,
            &[
                ("fileName", "Имя"),
                ("fileSize", "Размер, байт"),
                ("nbSeeders", "Сиды"),
                ("nbLeechers", "Личи"),
                ("siteUrl", "Источник"),
            ],
            |ui, row| {
                if ui.small_button("Добавить").clicked() {
                    self.add_sources.push(crate::add_torrent::Source::Link {
                        url: text(row, "fileUrl"),
                        downloader: text(row, "engineName"),
                    });
                }
                let url = text(row, "descrLink");
                if url.starts_with("https://") || url.starts_with("http://") {
                    ui.hyperlink_to("Описание", url);
                }
            },
        );
    }
    fn log_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.log_peers, false, "Основной журнал");
            ui.selectable_value(&mut self.log_peers, true, "Соединения пиров");
            ui.add(egui::TextEdit::singleline(&mut self.log_filter).hint_text("Фильтр"));
        });
        let key = if self.log_peers {
            "log/peers"
        } else {
            "log/main"
        };
        if let Some(data) = self.get(key, key, vec![("last_known_id".into(), "-1".into())]) {
            egui::ScrollArea::both().show(ui, |ui| {
                for entry in data.as_array().into_iter().flatten().rev() {
                    let message = if self.log_peers {
                        format!(
                            "{} · {} · {}",
                            text(entry, "ip"),
                            entry["blocked"],
                            text(entry, "reason")
                        )
                    } else {
                        text(entry, "message")
                    };
                    if !message
                        .to_lowercase()
                        .contains(&self.log_filter.to_lowercase())
                    {
                        continue;
                    }
                    let time = chrono::DateTime::from_timestamp_millis(number(entry, "timestamp"))
                        .map(|t| {
                            t.with_timezone(&chrono::Local)
                                .format("%d.%m %H:%M:%S")
                                .to_string()
                        })
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        ui.weak(time);
                        ui.add(egui::Label::new(message).wrap());
                    });
                    ui.separator();
                }
            });
        } else {
            self.loading(ui, key);
        }
    }
    fn categories_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Создать категорию").clicked() {
                self.open_form(
                    "Создать категорию",
                    "torrents/createCategory",
                    vec![],
                    vec![
                        Field::new("category", "Название", ""),
                        Field::new("savePath", "Путь на сервере", ""),
                    ],
                    "",
                );
            }
            if ui.button("Создать теги").clicked() {
                self.open_form(
                    "Создать теги",
                    "torrents/createTags",
                    vec![],
                    vec![Field::new("tags", "Теги через запятую", "")],
                    "",
                );
            }
        });
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(categories) = self.get("categories", "torrents/categories", vec![]) {
                for (name, c) in categories.as_object().into_iter().flatten() {
                    ui.horizontal_wrapped(|ui| {
                        ui.strong(name);
                        ui.label(text(c, "savePath"));
                        if ui.button("Изменить").clicked() {
                            self.open_form(
                                "Изменить категорию",
                                "torrents/editCategory",
                                vec![("category".into(), name.clone())],
                                vec![Field::new(
                                    "savePath",
                                    "Путь на сервере",
                                    text(c, "savePath"),
                                )],
                                "",
                            );
                        }
                        if ui.button("Удалить").clicked() {
                            self.open_form(
                                "Удалить категорию",
                                "torrents/removeCategories",
                                vec![("categories".into(), name.clone())],
                                vec![],
                                name,
                            );
                        }
                    });
                }
            } else {
                self.loading(ui, "categories");
            }
            ui.separator();
            ui.strong("Теги");
            if let Some(tags) = self.get("tags", "torrents/tags", vec![]) {
                for tag in tags
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                {
                    ui.horizontal(|ui| {
                        ui.label(tag);
                        if ui.button("Удалить").clicked() {
                            self.open_form(
                                "Удалить тег",
                                "torrents/deleteTags",
                                vec![("tags".into(), tag.into())],
                                vec![],
                                tag,
                            );
                        }
                    });
                }
            }
        });
    }
    fn preferences_page(&mut self, ui: &mut egui::Ui) {
        self.get("preferences", "app/preferences", vec![]);
        let Some(mut prefs) = self.prefs.take() else {
            self.loading(ui, "preferences");
            return;
        };
        ui.label("Параметры удалённого qBittorrent. Изменения вступают в силу после сохранения.");
        ui.horizontal(|ui|{
            ui.add(egui::TextEdit::singleline(&mut self.prefs_filter).hint_text("Найти параметр"));
            let diff=preference_diff(self.prefs_original.as_ref().unwrap_or(&Value::Null),&prefs);
            if ui.add_enabled(diff.as_object().is_some_and(|m|!m.is_empty()),egui::Button::new("Сохранить")).clicked(){self.open_form("Сохранить настройки","app/setPreferences",vec![("json".into(),diff.to_string())],vec![],"Применить изменённые параметры сервера? Настройки Web UI и сети могут изменить доступность подключения.");}
            if ui.button("Отменить изменения").clicked(){prefs=self.prefs_original.clone().unwrap_or_default();}
        });
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(map) = prefs.as_object_mut() {
                for (key, value) in map {
                    if !key
                        .to_lowercase()
                        .contains(&self.prefs_filter.to_lowercase())
                        && !label(key)
                            .to_lowercase()
                            .contains(&self.prefs_filter.to_lowercase())
                    {
                        continue;
                    }
                    ui.push_id(key, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.add_sized([260.0, 24.0], egui::Label::new(label(key)))
                                .on_hover_text(key);
                            edit_value(ui, key, value);
                        });
                    });
                    ui.separator();
                }
            }
        });
        self.prefs = Some(prefs);
    }
    fn rss_page(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.rss_rules, false, "Ленты и статьи");
            ui.selectable_value(&mut self.rss_rules, true, "Правила автозагрузки");
        });
        if self.rss_rules {
            let busy = self
                .pending
                .values()
                .any(|p| p.key == "Импорт RSS" || p.key == "Экспорт RSS");
            ui.add_enabled_ui(!busy, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Импорт правил…").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Правила RSS qBittorrent", &["json"])
                            .pick_file()
                        {
                            match crate::rss_rules::read(&path) {
                                Ok(rules) => self.rss_import = Some((rules, false)),
                                Err(e) => {
                                    self.message = e;
                                    self.error = true;
                                }
                            }
                        }
                    }
                    if ui.button("Экспорт правил…").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Правила RSS qBittorrent", &["json"])
                            .set_file_name("rss-downloader-rules.json")
                            .save_file()
                        {
                            self.rss_transfer(Some(path), Value::Null, false);
                        }
                    }
                });
            });
            if ui.button("Добавить правило").clicked() {
                self.rule_form("",&json!({"enabled":true,"mustContain":"","mustNotContain":"","useRegex":false,"episodeFilter":"","smartFilter":false,"ignoreDays":0,"affectedFeeds":[],"assignedCategory":"","savePath":"","addPaused":false}));
            }
            if let Some(rules) = self.get("rss-rules", "rss/rules", vec![]) {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (name, rule) in rules.as_object().into_iter().flatten() {
                        ui.collapsing(name, |ui| {
                            object_grid(ui, name, rule);
                            ui.horizontal(|ui| {
                                if ui.button("Копировать правило").clicked() {
                                    self.rule_form(&format!("{name} — копия"), rule);
                                }
                                if ui.button("Изменить").clicked() {
                                    self.rule_form(name, rule);
                                }
                                if ui.button("Удалить").clicked() {
                                    self.open_form(
                                        "Удалить правило",
                                        "rss/removeRule",
                                        vec![("ruleName".into(), name.clone())],
                                        vec![],
                                        name,
                                    );
                                }
                                if ui.button("Переименовать").clicked() {
                                    self.open_form(
                                        "Переименовать правило",
                                        "rss/renameRule",
                                        vec![("ruleName".into(), name.clone())],
                                        vec![Field::new("newRuleName", "Новое имя", name)],
                                        "",
                                    );
                                }
                            });
                            ui.collapsing("Подходящие статьи", |ui| {
                                let key = format!("rss-match/{name}");
                                if let Some(matches) = self.get(
                                    &key,
                                    "rss/matchingArticles",
                                    vec![("ruleName".into(), name.clone())],
                                ) {
                                    object_grid(ui, &key, &matches);
                                } else {
                                    self.loading(ui, &key);
                                }
                            });
                        });
                    }
                });
            } else {
                self.loading(ui, "rss-rules");
            }
        } else {
            ui.horizontal_wrapped(|ui| {
                if ui.button("Добавить ленту").clicked() {
                    self.open_form(
                        "Добавить ленту",
                        "rss/addFeed",
                        vec![],
                        vec![
                            Field::new("url", "URL", ""),
                            Field::new("path", "Название / путь ленты", ""),
                        ],
                        "",
                    );
                }
                if ui.button("Создать папку").clicked() {
                    self.open_form(
                        "Создать папку RSS",
                        "rss/addFolder",
                        vec![],
                        vec![Field::new("path", "Путь папки", "")],
                        "",
                    );
                }
                if ui.button("Обновить все").clicked() {
                    self.action(
                        "Обновить RSS",
                        "rss/refreshItem",
                        vec![("itemPath".into(), String::new())],
                    );
                }
            });
            if let Some(items) = self.get(
                "rss-items",
                "rss/items",
                vec![("withData".into(), "true".into())],
            ) {
                egui::ScrollArea::vertical().show(ui, |ui| self.rss_tree(ui, "", &items));
            } else {
                self.loading(ui, "rss-items");
            }
        }
    }
    fn rule_form(&mut self, name: &str, rule: &Value) {
        self.open_form(
            "Сохранить правило RSS",
            "rss/setRule",
            vec![],
            vec![
                Field::new("ruleName", "Имя правила", name),
                Field::new(
                    "ruleDef",
                    "Параметры правила",
                    serde_json::to_string_pretty(rule).unwrap_or_default(),
                ),
            ],
            "Укажите URL лент и условия для автоматической загрузки.",
        );
    }
    fn rss_tree(&mut self, ui: &mut egui::Ui, parent: &str, items: &Value) {
        for (name, item) in items.as_object().into_iter().flatten() {
            let path = if parent.is_empty() {
                name.clone()
            } else {
                format!("{parent}\\{name}")
            };
            ui.push_id(&path, |ui| {
                ui.collapsing(name, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Обновить").clicked() {
                            self.action(
                                "Обновить RSS",
                                "rss/refreshItem",
                                vec![("itemPath".into(), path.clone())],
                            );
                        }
                        if ui.button("Прочитано").clicked() {
                            self.action(
                                "Отметить прочитанным",
                                "rss/markAsRead",
                                vec![("itemPath".into(), path.clone())],
                            );
                        }
                        if ui.button("Переместить").clicked() {
                            self.open_form(
                                "Переместить RSS",
                                "rss/moveItem",
                                vec![("itemPath".into(), path.clone())],
                                vec![Field::new("destPath", "Новый путь / имя", &path)],
                                "",
                            );
                        }
                        if ui.button("Удалить").clicked() {
                            self.open_form(
                                "Удалить элемент RSS",
                                "rss/removeItem",
                                vec![("path".into(), path.clone())],
                                vec![],
                                &path,
                            );
                        }
                    });
                    if item.get("url").is_some() {
                        ui.label(text(item, "url"));
                        ui.horizontal(|ui| {
                            if ui.button("Изменить URL").clicked() {
                                self.open_form(
                                    "Изменить URL ленты",
                                    "rss/setFeedURL",
                                    vec![("path".into(), path.clone())],
                                    vec![Field::new("url", "URL", text(item, "url"))],
                                    "",
                                );
                            }
                            if ui.button("Интервал обновления").clicked() {
                                self.open_form(
                                    "Интервал RSS",
                                    "rss/setFeedRefreshInterval",
                                    vec![("path".into(), path.clone())],
                                    vec![Field::new(
                                        "refreshInterval",
                                        "Минуты · 0 по умолчанию",
                                        0,
                                    )],
                                    "",
                                );
                            }
                        });
                        for article in item["articles"].as_array().into_iter().flatten() {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(text(article, "title"));
                                let url = text(article, "torrentURL");
                                if !url.is_empty() && ui.small_button("Скачать").clicked() {
                                    self.add_sources.push(crate::add_torrent::Source::Link {
                                        url,
                                        downloader: String::new(),
                                    });
                                }
                            });
                        }
                    } else {
                        self.rss_tree(ui, &path, item);
                    }
                });
            });
        }
    }
}

fn preference_diff(original: &Value, edited: &Value) -> Value {
    Value::Object(
        edited
            .as_object()
            .into_iter()
            .flatten()
            .filter(|(k, v)| original.get(*k) != Some(*v))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    )
}
pub(super) fn edit_value(ui: &mut egui::Ui, key: &str, value: &mut Value) {
    match value {
        Value::Bool(b) => {
            ui.checkbox(b, "");
        }
        Value::String(s) => {
            ui.add(
                egui::TextEdit::singleline(s)
                    .password(key.contains("password") || key.contains("secret"))
                    .desired_width(340.0),
            );
        }
        Value::Number(n) => {
            if let Some(mut i) = n.as_i64() {
                if ui.add(egui::DragValue::new(&mut i)).changed() {
                    *n = i.into();
                }
            } else if let Some(mut f) = n.as_f64() {
                if ui.add(egui::DragValue::new(&mut f)).changed() {
                    if let Some(new) = serde_json::Number::from_f64(f) {
                        *n = new;
                    }
                }
            }
        }
        Value::Object(map) => {
            ui.vertical(|ui| {
                for (name, value) in map {
                    ui.push_id(name, |ui| {
                        ui.label(rss_label(name));
                        edit_value(ui, name, value);
                    });
                }
            });
        }
        Value::Array(items) => {
            ui.vertical(|ui| {
                let mut remove = None;
                for (index, item) in items.iter_mut().enumerate() {
                    ui.push_id(index, |ui| {
                        ui.horizontal(|ui| {
                            edit_value(ui, key, item);
                            if ui.small_button("−").clicked() {
                                remove = Some(index);
                            }
                        });
                    });
                }
                if let Some(index) = remove {
                    items.remove(index);
                }
                if ui.small_button("+ Добавить").clicked() {
                    items.push(Value::String(String::new()));
                }
            });
        }
        Value::Null => {
            ui.label("По умолчанию");
            if ui.small_button("Задать").clicked() {
                *value = Value::Bool(false);
            }
        }
    }
}
fn rss_label(key: &str) -> &str {
    match key {
        "enabled" => "Включено",
        "mustContain" => "Заголовок содержит",
        "mustNotContain" => "Заголовок не содержит",
        "useRegex" => "Регулярные выражения",
        "affectedFeeds" => "URL лент",
        "episodeFilter" => "Фильтр эпизодов",
        "smartFilter" => "Умный фильтр эпизодов",
        "ignoreDays" => "Не повторять загрузку, дней",
        "assignedCategory" | "category" => "Категория",
        "savePath" | "save_path" => "Путь на сервере",
        "addPaused" | "stopped" => "Добавлять остановленными",
        "torrentParams" => "Параметры торрента",
        "tags" => "Теги",
        _ => key,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_only_send_changed_fields() {
        assert_eq!(
            preference_diff(
                &json!({"a":1,"password":"","b":true}),
                &json!({"a":2,"password":"","b":true})
            ),
            json!({"a":2})
        );
    }
}
