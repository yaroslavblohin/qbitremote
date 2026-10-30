use crate::{api::Command, model::bytes};
use eframe::egui;
use serde_json::Value;
use std::{
    collections::VecDeque,
    path::PathBuf,
    time::{Duration, Instant},
};
mod tree;

#[derive(Clone, Debug)]
pub enum Source {
    File(PathBuf),
    Link { url: String, downloader: String },
}
impl Source {
    pub fn label(&self) -> String {
        match self {
            Self::File(p) => p.file_name().unwrap_or_default().to_string_lossy().into(),
            Self::Link { url, .. } => url.clone(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct File {
    pub path: String,
    pub size: u64,
    pub priority: u8,
}
#[derive(Clone, Debug)]
pub struct Metadata {
    pub hash: String,
    pub name: String,
    pub files: Vec<File>,
}
impl Metadata {
    pub fn parse(value: &Value) -> Result<Option<Self>, String> {
        if value.get("info").is_none() {
            return Ok(None);
        }
        let hash = value["hash"]
            .as_str()
            .or_else(|| value["infohash_v1"].as_str())
            .or_else(|| value["infohash_v2"].as_str())
            .ok_or("Сервер не вернул хеш торрента")?;
        if !matches!(hash.len(), 40 | 64) || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Сервер вернул неверный хеш торрента".into());
        }
        let files = value["info"]["files"]
            .as_array()
            .ok_or("Сервер не вернул список файлов")?
            .iter()
            .map(|f| {
                Ok(File {
                    path: f["path"].as_str().ok_or("Нет имени файла")?.into(),
                    size: f["length"].as_u64().ok_or("Нет размера файла")?,
                    priority: f["priority"]
                        .as_u64()
                        .filter(|p| matches!(p, 0 | 1 | 6 | 7))
                        .unwrap_or(1) as u8,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        if files.is_empty() {
            return Err("В торренте нет файлов".into());
        }
        Ok(Some(Self {
            hash: hash.to_lowercase(),
            name: value["info"]["name"].as_str().unwrap_or("Торрент").into(),
            files,
        }))
    }
}
#[derive(Clone)]
pub struct Plan {
    pub metadata: Metadata,
    pub path: String,
    pub category: String,
    pub tags: String,
    pub start: bool,
    pub top: bool,
    pub sequential: bool,
}
impl Plan {
    pub fn fields(&self) -> Result<Vec<(String, String)>, String> {
        if self.metadata.files.iter().all(|f| f.priority == 0) {
            return Err("Выберите хотя бы один файл".into());
        }
        if self
            .metadata
            .files
            .iter()
            .any(|f| !matches!(f.priority, 0 | 1 | 6 | 7))
        {
            return Err("Недопустимый приоритет файла".into());
        }
        Ok(vec![
            ("urls".into(), self.metadata.hash.clone()),
            (
                "filePriorities".into(),
                self.metadata
                    .files
                    .iter()
                    .map(|f| f.priority.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            ("stopped".into(), (!self.start).to_string()),
            ("paused".into(), (!self.start).to_string()),
            ("savepath".into(), self.path.clone()),
            ("category".into(), self.category.clone()),
            ("tags".into(), self.tags.clone()),
            ("autoTMM".into(), "false".into()),
            ("addToTopOfQueue".into(), self.top.to_string()),
            ("sequentialDownload".into(), self.sequential.to_string()),
        ])
    }
}
struct Draft {
    tree: tree::Tree,
    collapsed: std::collections::HashSet<usize>,
    filter: String,
    id: u64,
    source: Source,
    plan: Option<Plan>,
    loading: bool,
    committing: bool,
    message: String,
    next: Instant,
    started: Instant,
    path: String,
}
#[derive(Default)]
pub struct AddFlow {
    queue: VecDeque<(Source, String)>,
    current: Option<Draft>,
    serial: u64,
}
impl AddFlow {
    #[cfg(feature = "ui-smoke")]
    pub fn smoke_fixture(&mut self) {
        self.push(Source::File("Example.torrent".into()), "/downloads".into());
        let _ = self.tick(false);
        let mut value = serde_json::json!({"hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","info":{"name":"Открытые материалы — пример","files":[{"path":"Example/Video/film.mkv","length":2400000000_u64},{"path":"Example/Video/trailer.mkv","length":120000000},{"path":"Example/Subtitles/Русский.srt","length":150000},{"path":"Example/readme.txt","length":4000},{"path":"Example/Engine/Plugins/Codec/core.dll","length":25000000}]}});
        if std::env::var_os("QBIT_SMOKE_ADD_MANY").is_some() {
            for i in 0..200 {
                value["info"]["files"].as_array_mut().unwrap().push(serde_json::json!({"path":format!("Example/ZZ-Files/file-{i:03}.bin"),"length":1024000}));
            }
        }
        self.prepared(self.serial, Ok(value));
        let plan = self.current.as_mut().unwrap().plan.as_mut().unwrap();
        plan.metadata.files[0].priority = 6;
        plan.metadata.files[1].priority = 0;
        plan.metadata.files[2].priority = 7;
    }
    pub fn push(&mut self, source: Source, path: String) {
        self.queue.push_back((source, path));
    }
    pub fn active(&self) -> bool {
        self.current.is_some() || !self.queue.is_empty()
    }
    pub fn reset_session(&mut self) {
        if let Some(d) = self.current.take() {
            self.queue.push_front((d.source, d.path));
        }
    }
    pub fn tick(&mut self, connected: bool) -> Option<Command> {
        if self.current.is_none() {
            if let Some((source, path)) = self.queue.pop_front() {
                self.serial += 1;
                self.current = Some(Draft {
                    tree: Default::default(),
                    collapsed: Default::default(),
                    filter: String::new(),
                    id: self.serial,
                    source,
                    plan: None,
                    loading: false,
                    committing: false,
                    message: String::new(),
                    next: Instant::now(),
                    started: Instant::now(),
                    path,
                });
            }
        }
        let d = self.current.as_mut()?;
        if connected
            && d.plan.is_none()
            && !d.loading
            && d.message.is_empty()
            && Instant::now() >= d.next
        {
            d.loading = true;
            return Some(Command::PrepareTorrent {
                id: d.id,
                source: d.source.clone(),
            });
        }
        None
    }
    pub fn prepared(&mut self, id: u64, result: Result<Value, String>) {
        let Some(d) = self.current.as_mut().filter(|d| d.id == id) else {
            return;
        };
        d.loading = false;
        match result.and_then(|v| Metadata::parse(&v)) {
            Ok(Some(metadata)) => {
                d.tree = tree::Tree::new(&metadata.files);
                d.collapsed = d.tree.default_collapsed();
                d.plan = Some(Plan {
                    metadata,
                    path: d.path.clone(),
                    category: String::new(),
                    tags: String::new(),
                    start: false,
                    top: false,
                    sequential: false,
                });
            }
            Ok(None) => {
                if d.started.elapsed() > Duration::from_secs(120) {
                    d.message =
                        "Не удалось получить метаданные за 2 минуты. Можно повторить запрос."
                            .into();
                }
                d.next = Instant::now() + Duration::from_secs(2);
            }
            Err(e) => d.message = e,
        }
    }
    pub fn added(
        &mut self,
        id: u64,
        result: Result<String, String>,
    ) -> Option<Result<String, String>> {
        let d = self.current.as_mut().filter(|d| d.id == id)?;
        match &result {
            Ok(_) => self.current = None,
            Err(e) => {
                d.committing = false;
                d.message = e.clone();
            }
        }
        Some(result)
    }
    pub fn ui(&mut self, ctx: &egui::Context, connected: bool, server: &str) -> Option<Command> {
        let d = self.current.as_mut()?;
        let (mut cancel, mut submit, mut open) = (false, false, true);
        let _dialog = egui::Window::new("Добавить торрент — выбор файлов")
            .id(egui::Id::new("torrent-preview-tree"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .vscroll(false)
            .default_height(initial_height())
            .min_height(280.0)
            .max_height((ctx.screen_rect().height() - 70.0).max(160.0))
            .default_width((ctx.screen_rect().width() - 32.0).clamp(280.0, 760.0))
            .max_width((ctx.screen_rect().width() - 32.0).max(280.0))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                ui.spacing_mut().button_padding = egui::vec2(6.0, 2.0);
                ui.spacing_mut().interact_size.y = 18.0;
                ui.style_mut().override_font_id = Some(egui::FontId::proportional(12.0));
                ui.horizontal(|ui| {
                    ui.weak(format!("Сервер: {server}"));
                    if let Some(plan) = &d.plan {
                        ui.add(
                            egui::Label::new(egui::RichText::new(&plan.metadata.name).strong())
                                .truncate(),
                        )
                        .on_hover_text(d.source.label());
                    } else {
                        ui.add(egui::Label::new(d.source.label()).truncate())
                            .on_hover_text(d.source.label());
                    }
                });
                if !d.message.is_empty() {
                    ui.colored_label(egui::Color32::LIGHT_RED, &d.message);
                    if d.plan.is_none() && ui.button("Повторить получение метаданных").clicked()
                    {
                        d.message.clear();
                        d.started = Instant::now();
                    }
                }
                if let Some(plan) = &mut d.plan {
                    ui.add_enabled_ui(!d.committing, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Папка на сервере");
                            ui.add(
                                egui::TextEdit::singleline(&mut plan.path)
                                    .hint_text("По умолчанию")
                                    .desired_width((ui.available_width() - 10.0).max(100.0)),
                            );
                        });
                        let _options =
                            ui.collapsing("Дополнительные параметры", |ui| {
                                egui::Grid::new("add-options")
                                    .num_columns(2)
                                    .show(ui, |ui| {
                                        ui.label("Категория");
                                        ui.text_edit_singleline(&mut plan.category);
                                        ui.end_row();
                                        ui.label("Теги через запятую");
                                        ui.text_edit_singleline(&mut plan.tags);
                                        ui.end_row();
                                    });
                                ui.horizontal_wrapped(|ui| {
                                    ui.checkbox(&mut plan.top, "В начало очереди");
                                    ui.checkbox(&mut plan.sequential, "Последовательная загрузка");
                                });
                            });
                        #[cfg(test)]
                        ui.ctx().data_mut(|data| {
                            data.insert_temp(
                                egui::Id::new("add-options-expanded"),
                                _options.body_response.is_some(),
                            );
                            data.insert_temp(
                                egui::Id::new("add-options-header"),
                                _options.header_response.rect.center(),
                            )
                        });
                        let action = file_area(
                            ui,
                            plan,
                            &d.tree,
                            &mut d.collapsed,
                            &mut d.filter,
                            (d.id, connected, d.committing),
                        );
                        submit = action.0;
                        cancel = action.1;
                    });
                } else {
                    if connected {
                        ui.spinner();
                        ui.label(
                            "Получение списка файлов. Содержимое торрента ещё не загружается.",
                        );
                    } else {
                        ui.label("Подключите сервер для получения списка файлов.");
                    }
                    cancel = ui.button("Отмена").clicked();
                }
            });
        #[cfg(test)]
        if let Some(dialog) = _dialog {
            ctx.data_mut(|data| {
                data.insert_temp(egui::Id::new("add-dialog-rect"), dialog.response.rect)
            });
        }
        if (!open || cancel) && !d.committing {
            self.current = None;
            return None;
        }
        if submit {
            let d = self.current.as_mut().unwrap();
            let plan = d.plan.as_ref().unwrap();
            match plan.fields() {
                Ok(_) => {
                    d.committing = true;
                    d.message.clear();
                    return Some(Command::CommitTorrent {
                        id: d.id,
                        plan: Box::new(plan.clone()),
                    });
                }
                Err(error) => d.message = error,
            }
        }
        None
    }
}
fn initial_height() -> f32 {
    #[cfg(feature = "ui-smoke")]
    if let Some(height) = std::env::var("QBIT_SMOKE_ADD_HEIGHT")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .filter(|h| h.is_finite() && *h > 0.0)
    {
        return height;
    }
    500.0
}

fn priority_name(p: u8) -> &'static str {
    match p {
        0 => "Не скачивать",
        6 => "Высокий",
        7 => "Максимальный",
        _ => "Обычный",
    }
}

fn priority_text(palette: crate::theme::Palette, priority: u8) -> egui::RichText {
    let color = match priority {
        1 => palette.green,
        6 => palette.orange,
        7 => palette.red,
        _ => palette.muted,
    };
    egui::RichText::new(if priority == 255 {
        "Разный"
    } else {
        priority_name(priority)
    })
    .color(color)
}

fn file_area(
    ui: &mut egui::Ui,
    plan: &mut Plan,
    tree: &tree::Tree,
    collapsed: &mut std::collections::HashSet<usize>,
    filter: &mut String,
    options: (u64, bool, bool),
) -> (bool, bool) {
    let (id, connected, committing) = options;
    let (count, size) = plan
        .metadata
        .files
        .iter()
        .filter(|f| f.priority > 0)
        .fold((0_usize, 0_u64), |(count, size), f| {
            (count + 1, size.saturating_add(f.size))
        });
    let height = if ui.available_width() < 620.0 {
        72.0
    } else {
        48.0
    } + if committing { 21.0 } else { 0.0 };
    let footer = egui::TopBottomPanel::bottom(egui::Id::new(("add-torrent-footer", id)))
        .frame(egui::Frame::NONE)
        .show_separator_line(false)
        .exact_height(height)
        .show_inside(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.weak(format!(
                    "Выбрано {count} из {} · {}",
                    plan.metadata.files.len(),
                    bytes(size)
                ));
                ui.checkbox(&mut plan.start, "Начать загрузку после добавления");
            });
            let action = ui
                .horizontal(|ui| {
                    let submit_button = ui.add_enabled(
                        connected && count > 0,
                        egui::Button::new("Добавить торрент"),
                    );
                    let cancel_button = ui.button("Отмена");
                    #[cfg(test)]
                    ui.ctx().data_mut(|data| {
                        data.insert_temp(
                            egui::Id::new(("add-submit", id)),
                            submit_button.rect.center(),
                        );
                        data.insert_temp(
                            egui::Id::new(("add-cancel", id)),
                            cancel_button.rect.center(),
                        );
                    });
                    let submit = submit_button.clicked();
                    let cancel = cancel_button.clicked();
                    (submit, cancel)
                })
                .inner;
            if committing {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Добавление…");
                });
            }
            action
        });
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(egui::Id::new(("add-footer-rect", id)), footer.response.rect)
    });
    file_table(ui, &mut plan.metadata.files, tree, collapsed, filter, id);
    footer.inner
}

fn disclosure(ui: &mut egui::Ui, open: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(14.0, 18.0), egui::Sense::click());
    let c = rect.center();
    let points = if open {
        vec![
            c + egui::vec2(-4.0, -2.0),
            c + egui::vec2(4.0, -2.0),
            c + egui::vec2(0.0, 3.0),
        ]
    } else {
        vec![
            c + egui::vec2(-2.0, -4.0),
            c + egui::vec2(-2.0, 4.0),
            c + egui::vec2(3.0, 0.0),
        ]
    };
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        crate::theme::palette(ui.ctx()).orange,
        egui::Stroke::NONE,
    ));
    response
}

fn file_table(
    ui: &mut egui::Ui,
    files: &mut [File],
    tree: &tree::Tree,
    collapsed: &mut std::collections::HashSet<usize>,
    filter: &mut String,
    id: u64,
) {
    use egui_extras::{Column, TableBuilder};
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(filter)
                .hint_text("Найти файл или папку…")
                .desired_width(180.0),
        );
        if ui.small_button("Выбрать все").clicked() {
            for file in files.iter_mut() {
                file.priority = file.priority.max(1);
            }
        }
        if ui.small_button("Снять все").clicked() {
            for file in files.iter_mut() {
                file.priority = 0;
            }
        }
        if ui
            .small_button("Развернуть")
            .on_hover_text("Развернуть все папки")
            .clicked()
        {
            collapsed.clear();
        }
        if ui
            .small_button("Свернуть")
            .on_hover_text("Свернуть все папки")
            .clicked()
        {
            *collapsed = tree.folders();
        }
    });
    let rows = tree.visible(collapsed, filter);
    let stats = tree.stats(files);
    if rows.is_empty() {
        ui.weak("Нет файлов по этому фильтру");
        return;
    }
    egui::ScrollArea::horizontal().id_salt(("new-files-horizontal",id)).show(ui,|ui| {
        ui.set_min_width(480.0);
        let scroll_height=(ui.available_height()-25.0).max(0.0);
        let _table=TableBuilder::new(ui).id_salt(("new-torrent-files",id)).striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .min_scrolled_height(scroll_height).max_scroll_height(scroll_height).auto_shrink([false,false])
            .column(Column::remainder().at_least(235.0)).column(Column::initial(85.0)).column(Column::initial(125.0))
            .header(21.0,|mut h| {for label in ["Файлы","Размер","Приоритет"]{h.col(|ui| {ui.weak(label);});}})
            .body(|body| {body.rows(22.0,rows.len(),|mut row| {
                let node_id=rows[row.index()];let node=&tree.nodes[node_id];let state=stats[node_id];let folder=node.file.is_none();
                row.col(|ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x=3.0;ui.add_space(node.depth as f32*13.0);
                        if folder {
                            if disclosure(ui,!filter.trim().is_empty()||!collapsed.contains(&node_id)).clicked() && !collapsed.remove(&node_id){collapsed.insert(node_id);}
                        }else{ui.add_space(14.0);}
                        let mut selected=state.selected==node.count;
                        let mixed=state.selected>0&&state.selected<node.count;
                        let response=ui.add(egui::Checkbox::new(&mut selected,"").indeterminate(mixed)).on_hover_text(if folder{"Выбрать или исключить все файлы папки, включая скрытые фильтром"}else{"Скачать файл"});
                        #[cfg(test)]
                        ui.ctx().data_mut(|data|data.insert_temp(egui::Id::new(("add-tree-checkbox",id,node.path.clone())),response.rect.center()));
                        if response.changed(){tree.select(node_id,files,selected);}
                        let label=if folder{format!("{} ({})",node.name,node.count)}else{node.name.clone()};
                        let text=egui::RichText::new(label).color(if folder{crate::theme::palette(ui.ctx()).orange}else{ui.visuals().text_color()});
                        let response=ui.add(egui::Label::new(text).truncate().sense(egui::Sense::click())).on_hover_text(&node.path);
                        if folder&&response.double_clicked()&& !collapsed.remove(&node_id){collapsed.insert(node_id);}
                    });
                });
                row.col(|ui| {ui.label(bytes(node.size));});
                row.col(|ui| {
                    let mut priority=state.priority.unwrap_or(255);
                    egui::ComboBox::from_id_salt(("new-tree-prio",id,node_id)).width(115.0)
                        .selected_text(priority_text(crate::theme::palette(ui.ctx()),priority)).show_ui(ui,|ui| {
                            for p in [0,1,6,7]{ui.selectable_value(&mut priority,p,priority_text(crate::theme::palette(ui.ctx()),p));}
                        }).response.on_hover_text(if folder{"Приоритет применяется ко всем файлам папки, включая скрытые фильтром"}else{"Приоритет файла"});
                    if Some(priority)!=state.priority&&priority!=255{tree.priority(node_id,files,priority);}
                });
            });});
        #[cfg(test)]
        ui.ctx().data_mut(|data|data.insert_temp(egui::Id::new(("add-list-viewport",id)),_table.inner_rect));
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ready_flow(count: usize) -> AddFlow {
        let mut flow = AddFlow::default();
        flow.push(Source::File("test.torrent".into()), String::new());
        let _ = flow.tick(false);
        let files = (0..count)
            .map(|i| serde_json::json!({"path":format!("Game/Files/{i:04}.bin"),"length":100}))
            .collect::<Vec<_>>();
        flow.prepared(
            flow.serial,
            Ok(serde_json::json!({"hash":"a".repeat(40),"info":{"name":"Test","files":files}})),
        );
        flow
    }
    fn dialog_frame(
        ctx: &egui::Context,
        flow: &mut AddFlow,
        events: Vec<egui::Event>,
    ) -> Option<Command> {
        let mut command = None;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1100.0, 1100.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| command = flow.ui(ctx, true, "Test server"),
        );
        command
    }
    fn mouse(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]
    }
    #[test]
    fn expanded_dialog_resizes_via_native_egui_grip() {
        let ctx = egui::Context::default();
        ctx.style_mut(|s| s.animation_time = 0.0);
        let mut flow = ready_flow(300);
        for _ in 0..3 {
            assert!(dialog_frame(&ctx, &mut flow, vec![]).is_none());
        }
        let header = ctx
            .data(|data| data.get_temp::<egui::Pos2>(egui::Id::new("add-options-header")))
            .unwrap();
        let _ = dialog_frame(&ctx, &mut flow, mouse(header, true));
        let _ = dialog_frame(&ctx, &mut flow, mouse(header, false));
        let _ = dialog_frame(&ctx, &mut flow, vec![]);
        assert_eq!(
            ctx.data(|data| data.get_temp::<bool>(egui::Id::new("add-options-expanded"))),
            Some(true),
            "Options did not expand after the click"
        );
        let get = |key: &str| {
            ctx.data(|data| data.get_temp::<egui::Rect>(egui::Id::new((key, 1_u64))))
                .unwrap()
        };
        let before = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("add-dialog-rect")))
            .unwrap();
        let list_before = get("add-list-viewport");
        let footer_before = get("add-footer-rect");
        let grip = before.right_bottom() - egui::vec2(3.0, 3.0);
        let target = grip + egui::vec2(0.0, 160.0);
        let _ = dialog_frame(&ctx, &mut flow, mouse(grip, true));
        let _ = dialog_frame(&ctx, &mut flow, vec![egui::Event::PointerMoved(target)]);
        let _ = dialog_frame(&ctx, &mut flow, mouse(target, false));
        let _ = dialog_frame(&ctx, &mut flow, vec![]);
        let after = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("add-dialog-rect")))
            .unwrap();
        let list_after = get("add-list-viewport");
        let footer_after = get("add-footer-rect");
        assert!(
            after.height() - before.height() > 120.0,
            "Window did not resize: {before:?} -> {after:?}"
        );
        assert!(
            list_after.height() - list_before.height() > 120.0,
            "List did not follow resizing: {list_before:?} -> {list_after:?}"
        );
        assert!(
            (footer_after.bottom() - footer_before.bottom() - (after.height() - before.height()))
                .abs()
                < 3.0
        );
        assert!(list_after.bottom() <= footer_after.top() + 1.0);
        assert!(flow
            .current
            .as_ref()
            .unwrap()
            .plan
            .as_ref()
            .unwrap()
            .metadata
            .files
            .iter()
            .all(|f| f.priority == 1));
    }
    #[test]
    fn empty_search_keeps_selected_files_and_cancel_never_commits() {
        for cancel in [true, false] {
            let ctx = egui::Context::default();
            let mut flow = ready_flow(12);
            flow.current.as_mut().unwrap().filter = "not present".into();
            for _ in 0..3 {
                assert!(dialog_frame(&ctx, &mut flow, vec![]).is_none());
            }
            let key = if cancel { "add-cancel" } else { "add-submit" };
            let pos = ctx
                .data(|data| data.get_temp::<egui::Pos2>(egui::Id::new((key, 1_u64))))
                .unwrap();
            assert!(dialog_frame(&ctx, &mut flow, mouse(pos, true)).is_none());
            let command = dialog_frame(&ctx, &mut flow, mouse(pos, false));
            if cancel {
                assert!(command.is_none());
                assert!(!flow.active());
            } else {
                let Some(Command::CommitTorrent { plan, .. }) = command else {
                    panic!("Selected files were lost with empty search")
                };
                assert_eq!(plan.metadata.files.len(), 12);
                assert!(!plan.start);
                assert!(plan.metadata.files.iter().all(|f| f.priority == 1));
            }
        }
    }
    #[test]
    fn disabled_add_never_commits_when_nothing_selected_or_request_is_in_flight() {
        for committing in [false, true] {
            let ctx = egui::Context::default();
            let mut flow = ready_flow(8);
            let draft = flow.current.as_mut().unwrap();
            draft.committing = committing;
            if !committing {
                for file in &mut draft.plan.as_mut().unwrap().metadata.files {
                    file.priority = 0;
                }
            }
            for _ in 0..3 {
                assert!(dialog_frame(&ctx, &mut flow, vec![]).is_none());
            }
            let pos = ctx
                .data(|data| data.get_temp::<egui::Pos2>(egui::Id::new(("add-submit", 1_u64))))
                .unwrap();
            assert!(dialog_frame(&ctx, &mut flow, mouse(pos, true)).is_none());
            assert!(dialog_frame(&ctx, &mut flow, mouse(pos, false)).is_none());
            assert!(flow.active());
        }
    }
    #[test]
    fn resizing_grows_file_viewport_and_keeps_actions_at_bottom() {
        for width in [540.0, 900.0] {
            let mut measurements = Vec::new();
            for height in [360.0, 680.0] {
                let ctx = egui::Context::default();
                let files = (0..200)
                    .map(|i| File {
                        path: format!("Game/file-{i}.bin"),
                        size: 100,
                        priority: 1,
                    })
                    .collect::<Vec<_>>();
                let tree = tree::Tree::new(&files);
                let mut plan = Plan {
                    metadata: Metadata {
                        hash: "a".repeat(40),
                        name: "Example".into(),
                        files,
                    },
                    path: String::new(),
                    category: String::new(),
                    tags: String::new(),
                    start: false,
                    top: false,
                    sequential: false,
                };
                let mut collapsed = Default::default();
                let mut filter = String::new();
                for _ in 0..3 {
                    let _ = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(width, height),
                            )),
                            ..Default::default()
                        },
                        |ctx| {
                            egui::CentralPanel::default().show(ctx, |ui| {
                                let _ = file_area(
                                    ui,
                                    &mut plan,
                                    &tree,
                                    &mut collapsed,
                                    &mut filter,
                                    (77, true, false),
                                );
                            });
                        },
                    );
                }
                let footer = ctx
                    .data(|data| {
                        data.get_temp::<egui::Rect>(egui::Id::new(("add-footer-rect", 77_u64)))
                    })
                    .unwrap();
                let list = ctx
                    .data(|data| {
                        data.get_temp::<egui::Rect>(egui::Id::new(("add-list-viewport", 77_u64)))
                    })
                    .unwrap();
                assert!(
                    (footer.bottom() - (height - 8.0)).abs() < 2.0,
                    "footer not at bottom: {footer:?}"
                );
                assert!(
                    list.bottom() <= footer.top() + 1.0,
                    "list overlaps actions: {list:?} {footer:?}"
                );
                measurements.push((list.height(), footer.top()));
            }
            assert!(
                measurements[1].0 - measurements[0].0 > 300.0,
                "viewport did not grow: {measurements:?}"
            );
            assert!((measurements[1].1 - measurements[0].1 - 320.0).abs() < 2.0);
        }
    }
    #[test]
    fn clicking_mixed_folder_selects_all_without_changing_existing_high_priority() {
        let ctx = egui::Context::default();
        let mut files = vec![
            File {
                path: "Game/a".into(),
                size: 10,
                priority: 6,
            },
            File {
                path: "Game/b".into(),
                size: 20,
                priority: 0,
            },
            File {
                path: "other".into(),
                size: 5,
                priority: 1,
            },
        ];
        let tree = tree::Tree::new(&files);
        let mut collapsed = Default::default();
        let mut filter = String::new();
        let mut target = egui::Pos2::ZERO;
        for frame in 0..4 {
            let mut input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                ..Default::default()
            };
            if frame > 0 {
                input.events.push(egui::Event::PointerMoved(target));
                if frame < 3 {
                    input.events.push(egui::Event::PointerButton {
                        pos: target,
                        button: egui::PointerButton::Primary,
                        pressed: frame == 1,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
            }
            let _ = ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    file_table(ui, &mut files, &tree, &mut collapsed, &mut filter, 99)
                });
            });
            if frame == 0 {
                target = ctx
                    .data(|data| {
                        data.get_temp::<egui::Pos2>(egui::Id::new((
                            "add-tree-checkbox",
                            99_u64,
                            "Game".to_string(),
                        )))
                    })
                    .unwrap();
            }
        }
        assert_eq!(
            files.iter().map(|f| f.priority).collect::<Vec<_>>(),
            [6, 1, 1]
        );
    }
    pub fn metadata() -> Value {
        serde_json::json!({"hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","info":{"name":"Example","files":[{"path":"one.txt","length":10},{"path":"two.txt","length":20},{"path":"три.txt","length":30,"priority":0}]}})
    }
    #[test]
    fn priority_order_and_stopped_flag_are_sent_together() {
        let mut flow = AddFlow::default();
        flow.push(Source::File("test.torrent".into()), "/data".into());
        let _ = flow.tick(true);
        flow.prepared(flow.serial, Ok(metadata()));
        let plan = flow.current.as_mut().unwrap().plan.as_mut().unwrap();
        plan.metadata.files[0].priority = 6;
        plan.metadata.files[1].priority = 0;
        plan.metadata.files[2].priority = 7;
        let fields: std::collections::HashMap<_, _> = plan.fields().unwrap().into_iter().collect();
        assert_eq!(fields["filePriorities"], "6,0,7");
        assert_eq!(fields["stopped"], "true");
        assert_eq!(fields["savepath"], "/data");
        plan.start = true;
        assert!(plan
            .fields()
            .unwrap()
            .contains(&("stopped".into(), "false".into())));
        for f in &mut plan.metadata.files {
            f.priority = 0;
        }
        assert!(plan.fields().is_err());
    }
    #[test]
    fn preview_never_commits_and_stale_session_results_are_ignored() {
        let mut flow = AddFlow::default();
        flow.push(Source::File("test.torrent".into()), String::new());
        assert!(flow.tick(false).is_none());
        let old = flow.serial;
        assert!(matches!(
            flow.tick(true),
            Some(Command::PrepareTorrent { .. })
        ));
        assert!(flow.tick(true).is_none());
        flow.reset_session();
        let _ = flow.tick(true);
        flow.prepared(old, Ok(metadata()));
        assert!(flow.current.as_ref().unwrap().plan.is_none());
        flow.prepared(flow.serial, Ok(metadata()));
        assert!(flow.tick(true).is_none());
        let _ = flow.added(flow.serial, Err("Rejected".into()));
        assert!(flow.current.as_ref().unwrap().plan.is_some());
    }
    #[test]
    fn rejects_invalid_metadata() {
        assert!(Metadata::parse(&serde_json::json!({"info":{"files":[]},"hash":"bad"})).is_err());
        assert!(Metadata::parse(
            &serde_json::json!({"hash":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"})
        )
        .unwrap()
        .is_none());
    }
}
