use eframe::egui;
use std::collections::VecDeque;

#[derive(Default)]
pub struct Console {
    pub open: bool,
    entries: VecDeque<(String, bool)>,
}
impl Console {
    pub fn record(&mut self, message: String, error: bool) {
        if message.is_empty() {
            return;
        }
        if self.entries.len() >= 300 {
            self.entries.pop_front();
        }
        self.entries.push_back((
            format!("{}  {}", chrono::Local::now().format("%H:%M:%S"), message),
            error,
        ));
    }
    pub fn menu(&mut self, ui: &mut egui::Ui) {
        if ui.button("Консоль приложения…").clicked() {
            self.open = true;
            ui.close_menu();
        }
    }
    pub fn show(&mut self, ctx: &egui::Context) {
        egui::Window::new("Консоль приложения")
            .open(&mut self.open)
            .default_size(egui::vec2(640.0, 300.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Очистить").clicked() {
                        self.entries.clear();
                    }
                    if ui.button("Копировать").clicked() {
                        ui.ctx().copy_text(
                            self.entries
                                .iter()
                                .map(|(text, _)| text.as_str())
                                .collect::<Vec<_>>()
                                .join("\n"),
                        );
                    }
                });
                ui.separator();
                egui::ScrollArea::both()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if self.entries.is_empty() {
                            ui.weak("Сообщений пока нет");
                        }
                        for (text, error) in &self.entries {
                            let color = if *error {
                                crate::theme::palette(ctx).red
                            } else {
                                crate::theme::palette(ctx).text
                            };
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(text).monospace().color(color),
                                )
                                .selectable(true),
                            );
                        }
                    });
            });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn messages_do_not_open_console_and_history_is_bounded() {
        let mut console = Console::default();
        console.record(String::new(), false);
        assert!(console.entries.is_empty());
        for i in 0..400 {
            console.record(format!("Message {i}"), i % 2 == 0);
        }
        assert!(!console.open);
        assert_eq!(console.entries.len(), 300);
        assert!(console.entries.front().unwrap().0.ends_with("Message 100"));
        assert!(console.entries.back().unwrap().0.ends_with("Message 399"));
    }
}
