use super::*;

const GROUPS: [&str; 8] = [
    "Загрузки",
    "Скорость",
    "Соединение",
    "BitTorrent",
    "Очередь и раздача",
    "Web UI",
    "RSS",
    "Дополнительно",
];
fn group(key: &str) -> usize {
    if key.starts_with("web_ui") || key.starts_with("alternative_webui") {
        5
    } else if key.starts_with("rss") {
        6
    } else if key.contains("queue")
        || key.starts_with("max_active")
        || key.starts_with("dont_count")
        || key.contains("ratio")
        || key.contains("seeding_time")
    {
        4
    } else if key.contains("dl_limit")
        || key.contains("up_limit")
        || key.starts_with("scheduler")
        || key.starts_with("schedule")
        || key.starts_with("limit_")
    {
        1
    } else if key.starts_with("proxy")
        || key.contains("interface")
        || matches!(
            key,
            "listen_port"
                | "random_port"
                | "upnp"
                | "outgoing_ports_min"
                | "outgoing_ports_max"
                | "max_connec"
                | "max_connec_per_torrent"
        )
    {
        2
    } else if key.starts_with("announce")
        || matches!(
            key,
            "dht"
                | "pex"
                | "lsd"
                | "encryption"
                | "anonymous_mode"
                | "bittorrent_protocol"
                | "enable_embedded_tracker"
        )
    {
        3
    } else if key.starts_with("add_")
        || key.starts_with("scan_")
        || key.contains("save_path")
        || key.contains("temp_path")
        || key.contains("tmm")
        || key.contains("torrent_content_layout")
        || matches!(
            key,
            "preallocate_all"
                | "incomplete_files_ext"
                | "export_dir"
                | "export_dir_fin"
                | "create_subfolder_enabled"
                | "auto_delete_mode"
        )
    {
        0
    } else {
        7
    }
}
fn caption(key: &str) -> String {
    let translated = match key {
        "save_path" => "Папка загрузки по умолчанию",
        "temp_path" => "Папка незавершённых загрузок",
        "temp_path_enabled" => "Отдельная папка для незавершённых",
        "add_stopped_enabled" | "start_paused_enabled" => "Добавлять торренты остановленными",
        "add_to_top_of_queue" => "Новые торренты в начало очереди",
        "add_trackers" => "Дополнительные трекеры",
        "add_trackers_enabled" => "Добавлять дополнительные трекеры",
        "add_trackers_from_url_enabled" => "Получать трекеры по URL",
        "add_trackers_url" => "URL списка трекеров",
        "add_trackers_url_list" => "Список URL трекеров",
        "preallocate_all" => "Заранее выделять место для файлов",
        "incomplete_files_ext" => "Расширение .!qB для незавершённых",
        "auto_tmm_enabled" => "Автоматическое управление торрентами",
        "torrent_content_layout" => "Структура содержимого торрента",
        "export_dir" => "Экспорт .torrent при добавлении",
        "export_dir_fin" => "Экспорт .torrent после завершения",
        "scan_dirs" => "Наблюдаемые папки на сервере",
        "dl_limit" => "Лимит скачивания, Б/с",
        "up_limit" => "Лимит отдачи, Б/с",
        "alt_dl_limit" => "Альтернативное скачивание, Б/с",
        "alt_up_limit" => "Альтернативная отдача, Б/с",
        "scheduler_enabled" => "Расписание альтернативных лимитов",
        "schedule_from_hour" => "Начало расписания, час",
        "schedule_from_min" => "Начало расписания, минута",
        "schedule_to_hour" => "Конец расписания, час",
        "schedule_to_min" => "Конец расписания, минута",
        "scheduler_days" => "Дни расписания",
        "listen_port" => "Порт входящих соединений",
        "random_port" => "Случайный порт при запуске",
        "upnp" => "Проброс портов UPnP / NAT-PMP",
        "max_connec" => "Максимум соединений",
        "max_connec_per_torrent" => "Соединений на торрент",
        "proxy_type" => "Тип прокси",
        "proxy_ip" => "Адрес прокси",
        "proxy_port" => "Порт прокси",
        "proxy_auth_enabled" => "Авторизация прокси",
        "proxy_username" => "Имя пользователя прокси",
        "proxy_password" => "Пароль прокси",
        "current_network_interface" => "Сетевой интерфейс",
        "dht" => "Обмен пирами через DHT",
        "pex" => "Обмен пирами PeX",
        "lsd" => "Поиск локальных пиров LSD",
        "encryption" => "Режим шифрования",
        "anonymous_mode" => "Анонимный режим",
        "bittorrent_protocol" => "Протокол соединений",
        "announce_ip" => "IP для анонсов трекерам",
        "announce_port" => "Порт для анонсов",
        "announce_to_all_tiers" => "Анонсировать на всех уровнях",
        "announce_to_all_trackers" => "Анонсировать всем трекерам",
        "queueing_enabled" => "Очередь торрентов",
        "max_active_downloads" => "Активных загрузок",
        "max_active_uploads" => "Активных раздач",
        "max_active_torrents" => "Активных торрентов всего",
        "dont_count_slow_torrents" => "Не учитывать медленные торренты",
        "max_ratio_enabled" => "Ограничивать коэффициент раздачи",
        "max_ratio" => "Максимальный коэффициент раздачи",
        "max_seeding_time_enabled" => "Ограничивать время раздачи",
        "max_seeding_time" => "Время раздачи, минуты",
        "web_ui_address" => "Адрес Web UI",
        "web_ui_port" => "Порт Web UI",
        "web_ui_username" => "Имя пользователя Web UI",
        "web_ui_password" => "Пароль Web UI",
        "web_ui_upnp" => "Проброс порта Web UI",
        "web_ui_https_enabled" => "Использовать HTTPS",
        "web_ui_csrf_protection_enabled" => "Защита от CSRF",
        "web_ui_clickjacking_protection_enabled" => "Защита от подмены интерфейса",
        "web_ui_host_header_validation_enabled" => "Проверять заголовок Host",
        "web_ui_localhost_auth" => "Авторизация для localhost",
        "web_ui_auth_subnet_whitelist_enabled" => "Исключения авторизации по подсети",
        "web_ui_auth_subnet_whitelist" => "Подсети без авторизации",
        "alternative_webui_enabled" => "Альтернативный Web UI",
        "alternative_webui_path" => "Папка альтернативного Web UI",
        "rss_processing_enabled" => "Обработка RSS",
        "rss_auto_downloading_enabled" => "Автозагрузка RSS",
        "rss_refresh_interval" => "Обновление RSS, минуты",
        "rss_max_articles_per_feed" => "Статей в одной RSS-ленте",
        "app_instance_name" => "Имя экземпляра сервера",
        _ => label(key),
    };
    if translated != key {
        translated.into()
    } else {
        let name = key.replace('_', " ");
        let mut chars = name.chars();
        chars.next().map_or(String::new(), |first| {
            first.to_uppercase().collect::<String>() + chars.as_str()
        })
    }
}
fn matches(key: &str, query: &str) -> bool {
    query.is_empty()
        || key.to_lowercase().contains(query)
        || caption(key).to_lowercase().contains(query)
}
pub(super) fn render(
    ui: &mut egui::Ui,
    prefs: &mut Value,
    selected_group: &mut usize,
    filter: &str,
) {
    let Some(map) = prefs.as_object_mut() else {
        return;
    };
    let query = filter.trim().to_lowercase();
    let counts: Vec<_> = (0..GROUPS.len())
        .map(|index| map.keys().filter(|key| group(key) == index).count())
        .collect();
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(154.0, ui.available_height()),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_min_width(154.0);
                for (index, name) in GROUPS.iter().enumerate() {
                    ui.add_enabled_ui(counts[index] > 0, |ui| {
                        ui.selectable_value(selected_group, index, *name);
                    });
                }
            },
        );
        ui.separator();
        let width = ui.available_width().min(780.0);
        egui::ScrollArea::vertical()
            .id_salt("server-preferences-fields")
            .auto_shrink([false, false])
            .max_width(width)
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.set_width(width);
                    ui.spacing_mut().item_spacing.y = 3.0;
                    ui.spacing_mut().interact_size.y = 22.0;
                    ui.label(
                        RichText::new(if query.is_empty() {
                            GROUPS[(*selected_group).min(GROUPS.len() - 1)]
                        } else {
                            "Результаты поиска"
                        })
                        .strong(),
                    );
                    ui.add_space(5.0);
                    let mut shown = 0;
                    for (key, value) in map {
                        if (query.is_empty() && group(key) != *selected_group)
                            || !matches(key, &query)
                        {
                            continue;
                        }
                        shown += 1;
                        ui.push_id(key, |ui| field(ui, key, value));
                    }
                    if shown == 0 {
                        ui.weak(if query.is_empty() {
                            "В этом разделе нет параметров"
                        } else {
                            "Параметры не найдены"
                        });
                    }
                });
            });
    });
}
fn field(ui: &mut egui::Ui, key: &str, value: &mut Value) {
    let width = ui.available_width();
    let label_width = (width * 0.4).clamp(110.0, 270.0).min(width * 0.55);
    let control_width = (width - label_width - ui.spacing().item_spacing.x).max(50.0);
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(label_width, 22.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_min_width(label_width);
                ui.add(egui::Label::new(caption(key)).truncate())
                    .on_hover_text(format!("{}\n{key}", caption(key)));
            },
        );
        ui.allocate_ui_with_layout(
            egui::vec2(control_width, 22.0),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.set_width(control_width);
                match value {
                    Value::Bool(v) => {
                        ui.checkbox(v, "");
                    }
                    Value::String(v) => {
                        ui.add(
                            egui::TextEdit::singleline(v)
                                .desired_width(control_width)
                                .password(key.contains("password") || key.contains("secret")),
                        );
                    }
                    Value::Number(v) => {
                        if let Some(mut number) = v.as_i64() {
                            if ui
                                .add_sized(
                                    egui::vec2(104.0, 22.0),
                                    egui::DragValue::new(&mut number),
                                )
                                .changed()
                            {
                                *v = number.into();
                            }
                        } else if let Some(mut number) = v.as_f64() {
                            if ui
                                .add_sized(
                                    egui::vec2(104.0, 22.0),
                                    egui::DragValue::new(&mut number),
                                )
                                .changed()
                            {
                                if let Some(new) = serde_json::Number::from_f64(number) {
                                    *v = new;
                                }
                            }
                        }
                    }
                    Value::Array(_) | Value::Object(_) => {
                        ui.collapsing("Редактировать список…", |ui| {
                            super::pages::edit_value(ui, key, value)
                        });
                    }
                    Value::Null => {
                        ui.weak("—");
                    }
                }
            },
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_are_categorized_and_searchable_by_name_and_api_key() {
        for (key, expected) in [
            ("save_path", 0),
            ("alt_dl_limit", 1),
            ("listen_port", 2),
            ("dht", 3),
            ("add_to_top_of_queue", 4),
            ("web_ui_port", 5),
            ("rss_processing_enabled", 6),
            ("future_server_field", 7),
        ] {
            assert_eq!(group(key), expected);
        }
        assert!(matches("save_path", "папка"));
        assert!(matches("alt_dl_limit", "alt_dl"));
        assert!(!matches("dht", "пароль"));
        assert!(!caption("future_server_field").contains('_'));
    }
    #[test]
    fn displaying_all_setting_types_and_search_does_not_change_values() {
        let original = json!({"save_path":"/downloads","temp_path_enabled":true,"dl_limit":0,"listen_port":6881,"encryption":1,"max_active_downloads":3,"web_ui_password":"hidden","rss_refresh_interval":20,"future_server_field":1.5,"scan_dirs":{"/watch":0},"rss_rules":["one","two"],"empty":null});
        let ctx = egui::Context::default();
        for group in 0..GROUPS.len() {
            for query in ["", "папка", "web_ui", "not-found"] {
                let mut prefs = original.clone();
                let mut selected = group;
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(1100.0, 700.0),
                        )),
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default()
                            .show(ctx, |ui| render(ui, &mut prefs, &mut selected, query));
                    },
                );
                assert_eq!(prefs, original);
            }
        }
    }
    #[test]
    fn compact_fields_align_and_use_one_row_without_separators() {
        for scale in [1.0, 1.5, 2.0] {
            let ctx = egui::Context::default();
            ctx.set_zoom_factor(scale);
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0 * scale, 500.0 * scale),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let mut prefs = json!({"save_path":"/downloads", "temp_path":"/incomplete", "add_stopped_enabled":true});
                        render(ui, &mut prefs, &mut 0, "");
                    });
                },
            );
            let position = |name: &str| {
                output
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(text) if text.galley.text() == name => Some(text.pos),
                        _ => None,
                    })
                    .unwrap()
            };
            let a = position(&caption("save_path"));
            let b = position(&caption("temp_path"));
            let c = position(&caption("add_stopped_enabled"));
            assert!((a.x - b.x).abs() < 0.1 && (b.x - c.x).abs() < 0.1);
            let mut y = [a.y, b.y, c.y];
            y.sort_by(f32::total_cmp);
            assert!(
                y.windows(2)
                    .all(|pair| pair[1] - pair[0] >= 20.0 && pair[1] - pair[0] <= 30.0),
                "settings must form compact vertical rows"
            );
        }
    }
}
