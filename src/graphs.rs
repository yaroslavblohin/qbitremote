use crate::model::{bytes, Torrent};
use eframe::egui;
use std::{collections::HashMap, time::Instant};

#[derive(Clone, Copy)]
struct Sample {
    seconds: f64,
    down: f64,
    up: f64,
}
#[derive(Default)]
struct View {
    span: Option<f64>,
    end: Option<f64>,
    y_zoom: f64,
}
impl View {
    fn reset(&mut self) {
        *self = Self {
            span: None,
            end: None,
            y_zoom: 1.0,
        };
    }
    fn range(&self, last: f64) -> (f64, f64) {
        let span = self.span.unwrap_or(last.max(3.0)).max(3.0);
        let end = self.end.unwrap_or(last).clamp(0.0, last.max(0.0));
        let start = (end - span).max(0.0);
        (start, start + span)
    }
    fn zoom_time(&mut self, factor: f64, last: f64) {
        self.span = Some((self.span.unwrap_or(last.max(3.0)) * factor).clamp(3.0, last.max(60.0)));
    }
}
struct Channel {
    started: f64,
    samples: Vec<Sample>,
    view: View,
}
impl Channel {
    fn new(started: f64) -> Self {
        let mut view = View::default();
        view.reset();
        Self {
            started,
            samples: Vec::new(),
            view,
        }
    }
    fn push(&mut self, seconds: f64, down: f64, up: f64) {
        self.samples.push(Sample {
            seconds: (seconds - self.started).max(0.0),
            down,
            up,
        });
    }
}
pub struct Graphs {
    started: Instant,
    total: Option<Channel>,
    torrents: HashMap<String, Channel>,
}
impl Default for Graphs {
    fn default() -> Self {
        Self {
            started: Instant::now(),
            total: None,
            torrents: HashMap::new(),
        }
    }
}
impl Graphs {
    fn enabled(&self, hash: Option<&str>) -> bool {
        hash.map_or(self.total.is_some(), |hash| {
            self.torrents.contains_key(hash)
        })
    }
    fn set_enabled(&mut self, hash: Option<&str>, enabled: bool) {
        self.set_enabled_at(hash, enabled, self.started.elapsed().as_secs_f64());
    }
    fn set_enabled_at(&mut self, hash: Option<&str>, enabled: bool, seconds: f64) {
        if self.enabled(hash) == enabled {
            return;
        }
        if let Some(hash) = hash {
            if enabled {
                self.torrents.insert(hash.to_owned(), Channel::new(seconds));
            } else {
                self.torrents.remove(hash);
                if self.torrents.is_empty() {
                    self.torrents = HashMap::new();
                }
            }
        } else {
            self.total = enabled.then(|| Channel::new(seconds));
        }
    }
    pub fn clear_history(&mut self) {
        self.total = None;
        self.torrents = HashMap::new();
        self.started = Instant::now();
    }
    pub fn record(&mut self, torrents: &[Torrent]) {
        if self.total.is_none() && self.torrents.is_empty() {
            return;
        }
        self.record_at(torrents, self.started.elapsed().as_secs_f64());
    }
    fn record_at(&mut self, torrents: &[Torrent], seconds: f64) {
        if self.total.is_none() && self.torrents.is_empty() {
            return;
        }
        let mut down = 0.0;
        let mut up = 0.0;
        for torrent in torrents {
            if let Some(channel) = self.torrents.get_mut(&torrent.hash) {
                channel.push(seconds, torrent.dlspeed as f64, torrent.upspeed as f64);
            }
            if self.total.is_some() {
                down += torrent.dlspeed as f64;
                up += torrent.upspeed as f64;
            }
        }
        if let Some(channel) = &mut self.total {
            channel.push(seconds, down, up);
        }
    }
    #[cfg(feature = "ui-smoke")]
    pub fn demo(&mut self, hash: Option<&str>) {
        self.set_enabled_at(hash, true, 0.0);
        for i in 0..60 {
            self.record_at(
                &[Torrent {
                    hash: "fixture-1".into(),
                    dlspeed: (8_000_000.0 + (i as f64 * 0.3).sin() * 3_000_000.0) as u64,
                    upspeed: 2_000_000,
                    ..Default::default()
                }],
                i as f64 * 3.0,
            );
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, hash: Option<&str>) {
        let mut enabled = self.enabled(hash);
        let label = if hash.is_some() {
            "Записывать график этого торрента"
        } else {
            "Записывать общий график скорости"
        };
        let hint = if hash.is_some() {
            "Включает запись только этого торрента. Остальные торренты и общий график не затрагиваются. Выключение освобождает только эту историю."
        } else {
            "Включает запись только суммарной скорости. Индивидуальные графики торрентов не затрагиваются."
        };
        if ui
            .checkbox(&mut enabled, label)
            .on_hover_text(hint)
            .changed()
        {
            self.set_enabled(hash, enabled);
        }
        let channel = if let Some(hash) = hash {
            self.torrents.get_mut(hash)
        } else {
            self.total.as_mut()
        };
        let Some(channel) = channel else {
            ui.weak("График выключен. История не собирается.");
            return;
        };
        let history = channel.samples.as_slice();
        let view = &mut channel.view;
        let last = history.last().map_or(0.0, |sample| sample.seconds);
        ui.horizontal_wrapped(|ui| {
            ui.label("Время");
            egui::ComboBox::from_id_salt("graph-time-range")
                .selected_text(view.span.map_or("Вся история".into(), duration))
                .width(104.0)
                .show_ui(ui, |ui| {
                    for (name, span) in [
                        ("Вся история", None),
                        ("1 минута", Some(60.0)),
                        ("5 минут", Some(300.0)),
                        ("15 минут", Some(900.0)),
                        ("1 час", Some(3600.0)),
                    ] {
                        if ui.selectable_value(&mut view.span, span, name).changed() {
                            view.end = None;
                        }
                    }
                });
            if ui
                .small_button("−")
                .on_hover_text("Уменьшить масштаб времени")
                .clicked()
            {
                view.zoom_time(2.0, last);
            }
            if ui
                .small_button("+")
                .on_hover_text("Увеличить масштаб времени")
                .clicked()
            {
                view.zoom_time(0.5, last);
            }
            ui.separator();
            ui.label("Скорость");
            if ui
                .small_button("−")
                .on_hover_text("Уменьшить масштаб скорости")
                .clicked()
            {
                view.y_zoom = (view.y_zoom / 2.0).max(0.125);
            }
            if ui
                .small_button("+")
                .on_hover_text("Увеличить масштаб скорости")
                .clicked()
            {
                view.y_zoom = (view.y_zoom * 2.0).min(64.0);
            }
            if ui.small_button("Сброс").clicked() {
                view.reset();
            }
            if ui.small_button("К концу").clicked() {
                view.end = None;
            }
        });
        plot(ui, history, view);
        if history.is_empty() {
            ui.weak("Ожидание первого замера от сервера…");
        }
    }
}
fn duration(seconds: f64) -> String {
    let seconds = seconds.max(0.0).round() as u64;
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
}
fn plot(ui: &mut egui::Ui, history: &[Sample], view: &mut View) {
    let palette = crate::theme::palette(ui.ctx());
    ui.horizontal(|ui| {
        ui.colored_label(palette.blue, "↓ Скачивание");
        ui.colored_label(palette.green, "↑ Отдача");
    });
    let height = (ui.clip_rect().bottom() - ui.next_widget_position().y - 4.0).clamp(100.0, 180.0);
    let (outer, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(130.0), height),
        egui::Sense::drag(),
    );
    let rect = egui::Rect::from_min_max(
        outer.min + egui::vec2(78.0, 8.0),
        outer.max - egui::vec2(12.0, 25.0),
    );
    let last = history.last().map_or(0.0, |s| s.seconds);
    if response.hovered() {
        let (scroll, ctrl) = ui.input(|i| (i.smooth_scroll_delta.y, i.modifiers.ctrl));
        if scroll != 0.0 {
            let factor = (-scroll as f64 * 0.005).exp().clamp(0.5, 2.0);
            if ctrl {
                view.y_zoom = (view.y_zoom / factor).clamp(0.125, 64.0);
            } else {
                view.zoom_time(factor, last);
            }
            ui.input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
        }
    }
    if response.dragged() {
        let (start, end) = view.range(last);
        let delta = ui.input(|i| i.pointer.delta().x) as f64 / rect.width() as f64 * (end - start);
        view.end =
            Some((view.end.unwrap_or(last) - delta).clamp((end - start).min(last), last.max(0.0)));
    }
    response.on_hover_text("Колесо — масштаб времени. Ctrl + колесо — масштаб скорости. Перетаскивание — просмотр прошлых участков. «К концу» — следить за новыми замерами.");
    let (start, end) = view.range(last);
    let visible: Vec<_> = history
        .iter()
        .filter(|s| s.seconds >= start && s.seconds <= end)
        .copied()
        .collect();
    let peak = visible
        .iter()
        .fold(1024.0_f64, |peak, s| peak.max(s.down).max(s.up));
    let top = peak * 1.1 / view.y_zoom.max(0.125);
    let painter = ui.painter();
    painter.rect_filled(outer, 4.0, palette.input);
    for i in 0..=4 {
        let f = i as f32 / 4.0;
        let y = rect.bottom() - f * rect.height();
        let x = rect.left() + f * rect.width();
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0_f32, palette.border),
        );
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0_f32, palette.border),
        );
        painter.text(
            egui::pos2(rect.left() - 6.0, y),
            egui::Align2::RIGHT_CENTER,
            format!("{}/с", bytes((top * f as f64) as u64)),
            egui::FontId::proportional(10.0),
            palette.muted,
        );
        painter.text(
            egui::pos2(x, rect.bottom() + 7.0),
            if i == 0 {
                egui::Align2::LEFT_TOP
            } else if i == 4 {
                egui::Align2::RIGHT_TOP
            } else {
                egui::Align2::CENTER_TOP
            },
            if end - start < 10.0 {
                format!("{:.1} с", start + f as f64 * (end - start))
            } else {
                duration(start + f as f64 * (end - start))
            },
            egui::FontId::proportional(10.0),
            palette.muted,
        );
    }
    let clipped = painter.with_clip_rect(rect);
    for (series, color) in [(0, palette.blue), (1, palette.green)] {
        let points = points(&visible, rect, series, start, end, top);
        if points.len() > 1 {
            clipped.add(egui::Shape::line(points, egui::Stroke::new(1.0_f32, color)));
        } else if let Some(point) = points.first() {
            clipped.circle_filled(*point, 2.0, color);
        }
    }
}
fn points(
    history: &[Sample],
    rect: egui::Rect,
    series: usize,
    start: f64,
    end: f64,
    top: f64,
) -> Vec<egui::Pos2> {
    let bins = rect.width().ceil().max(1.0) as usize;
    let mut peaks: Vec<Option<(f64, f64)>> = vec![None; bins];
    for sample in history {
        if sample.seconds < start || sample.seconds > end {
            continue;
        }
        let bin = (((sample.seconds - start) / (end - start).max(0.001) * bins as f64) as usize)
            .min(bins - 1);
        let speed = if series == 0 { sample.down } else { sample.up };
        if peaks[bin].is_none_or(|(_, old)| speed > old) {
            peaks[bin] = Some((sample.seconds, speed));
        }
    }
    peaks
        .into_iter()
        .flatten()
        .map(|(time, speed)| {
            egui::pos2(
                rect.left() + ((time - start) / (end - start).max(0.001)) as f32 * rect.width(),
                rect.bottom() - (speed / top.max(1.0)) as f32 * rect.height(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn torrent() -> Torrent {
        Torrent {
            hash: "a".into(),
            dlspeed: 100,
            upspeed: 20,
            ..Default::default()
        }
    }
    #[test]
    fn disabled_by_default_does_not_allocate_history() {
        let mut graphs = Graphs::default();
        for _ in 0..100 {
            graphs.record(&[torrent()]);
        }
        assert!(graphs.total.is_none() && graphs.torrents.capacity() == 0);
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| graphs.ui(ui, Some("a")));
        });
        assert!(graphs.total.is_none() && graphs.torrents.capacity() == 0);
    }
    #[test]
    fn enabled_torrent_records_only_itself_and_off_frees_history() {
        let mut graphs = Graphs::default();
        graphs.set_enabled_at(Some("a"), true, 0.0);
        for i in 0..240 {
            graphs.record_at(
                &[
                    torrent(),
                    Torrent {
                        hash: "b".into(),
                        dlspeed: 300,
                        ..Default::default()
                    },
                ],
                i as f64 * 3.0,
            );
        }
        assert_eq!(graphs.torrents["a"].samples.len(), 240);
        assert_eq!(graphs.torrents["a"].samples[239].seconds, 717.0);
        assert!(graphs.total.is_none() && !graphs.enabled(Some("b")));
        graphs.set_enabled(Some("a"), false);
        assert_eq!(graphs.torrents.capacity(), 0);
        graphs.record(&[torrent()]);
        assert!(graphs.torrents.is_empty());
        graphs.set_enabled_at(Some("a"), true, 5.0);
        graphs.record_at(&[torrent()], 8.0);
        assert_eq!(graphs.torrents["a"].samples.len(), 1);
        assert_eq!(graphs.torrents["a"].samples[0].seconds, 3.0);
        graphs.clear_history();
        assert!(graphs.total.is_none() && graphs.torrents.is_empty());
    }
    #[test]
    fn independent_switches_clocks_and_scales_preserve_other_histories() {
        let mut graphs = Graphs::default();
        graphs.set_enabled_at(Some("a"), true, 10.0);
        graphs.record_at(&[torrent()], 13.0);
        graphs.torrents.get_mut("a").unwrap().view.span = Some(60.0);
        graphs.set_enabled_at(Some("b"), true, 20.0);
        graphs.set_enabled_at(None, true, 25.0);
        let torrents = [
            torrent(),
            Torrent {
                hash: "b".into(),
                dlspeed: 300,
                ..Default::default()
            },
        ];
        graphs.record_at(&torrents, 28.0);
        assert_eq!(graphs.torrents["a"].samples[1].seconds, 18.0);
        assert_eq!(graphs.torrents["b"].samples[0].seconds, 8.0);
        assert_eq!(graphs.total.as_ref().unwrap().samples[0].seconds, 3.0);
        assert_eq!(graphs.total.as_ref().unwrap().samples[0].down, 400.0);
        graphs.set_enabled(None, false);
        graphs.set_enabled(Some("b"), false);
        assert_eq!(graphs.torrents["a"].samples.len(), 2);
        assert_eq!(graphs.torrents["a"].view.span, Some(60.0));
        graphs.record_at(&torrents, 31.0);
        assert_eq!(graphs.torrents["a"].samples.len(), 3);
        assert!(graphs.total.is_none() && !graphs.enabled(Some("b")));
    }
    #[test]
    fn graph_opening_and_switching_does_not_enable_or_record_other_graphs() {
        let mut graphs = Graphs::default();
        graphs.set_enabled_at(Some("a"), true, 0.0);
        graphs.record_at(&[torrent()], 7.5);
        let ctx = egui::Context::default();
        for hash in [Some("a"), Some("b"), None, Some("a")] {
            let _ = ctx.run(Default::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| graphs.ui(ui, hash));
            });
        }
        assert!(graphs.total.is_none());
        assert_eq!(graphs.torrents["a"].samples.len(), 1);
        assert_eq!(graphs.torrents.len(), 1);
    }
    #[test]
    fn zoom_limits_and_time_ranges_are_valid() {
        let mut view = View::default();
        view.reset();
        assert_eq!(view.range(120.0), (0.0, 120.0));
        view.zoom_time(0.5, 120.0);
        assert_eq!(view.range(120.0), (60.0, 120.0));
        view.end = Some(90.0);
        assert_eq!(view.range(120.0), (30.0, 90.0));
        for _ in 0..100 {
            view.zoom_time(0.5, 120.0);
        }
        assert_eq!(view.span, Some(3.0));
        assert_eq!(duration(3661.0), "1:01:01");
    }
    #[test]
    fn decimation_preserves_peaks_and_actual_timestamps() {
        let mut history = vec![
            Sample {
                seconds: 0.0,
                down: 0.0,
                up: 0.0
            };
            10000
        ];
        for (i, sample) in history.iter_mut().enumerate() {
            sample.seconds = i as f64 * 0.7;
        }
        history[321].down = 100.0;
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(300.0, 150.0));
        let points = points(&history, rect, 0, 0.0, 7000.0, 100.0);
        assert!(points.len() <= 300 && points.iter().all(|p| rect.contains(*p)));
        assert!(points.iter().any(|p| p.y == rect.top()));
        let irregular = [Sample {
            seconds: 10.0,
            down: 50.0,
            up: 0.0,
        }];
        assert_eq!(
            super::points(&irregular, rect, 0, 0.0, 100.0, 100.0)[0].x,
            30.0
        );
    }
}
