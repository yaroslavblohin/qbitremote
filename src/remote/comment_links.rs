use eframe::egui;
use std::{ops::Range, sync::Arc};

#[derive(Debug, PartialEq)]
struct Link {
    bytes: Range<usize>,
    chars: Range<usize>,
}
fn links(text: &str) -> Vec<Link> {
    let mut result = Vec::new();
    let mut consumed = 0;
    for (start, _) in text.char_indices() {
        if start < consumed {
            continue;
        }
        let rest = &text[start..];
        if !["http://", "https://", "magnet:?", "udp://"]
            .iter()
            .any(|prefix| {
                rest.get(..prefix.len())
                    .is_some_and(|s| s.eq_ignore_ascii_case(prefix))
            })
        {
            continue;
        }
        let mut end = start
            + rest
                .find(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '\"' | '\''))
                .unwrap_or(rest.len());
        loop {
            let candidate = &text[start..end];
            let Some(last) = candidate.chars().last() else {
                break;
            };
            let unmatched = match last {
                ')' => candidate.matches(')').count() > candidate.matches('(').count(),
                ']' => candidate.matches(']').count() > candidate.matches('[').count(),
                '}' => candidate.matches('}').count() > candidate.matches('{').count(),
                _ => false,
            };
            if matches!(last, '.' | ',' | ';') || unmatched {
                end -= last.len_utf8();
            } else {
                break;
            }
        }
        consumed = end;
        if url::Url::parse(&text[start..end]).is_ok_and(|u| match u.scheme() {
            "http" | "https" | "udp" => u.host_str().is_some(),
            "magnet" => u
                .query_pairs()
                .any(|(key, value)| key == "xt" && !value.is_empty()),
            _ => false,
        }) {
            let chars = text[..start].chars().count();
            result.push(Link {
                bytes: start..end,
                chars: chars..chars + text[start..end].chars().count(),
            });
        }
    }
    result
}
pub fn layout(ui: &egui::Ui, text: &str, width: f32) -> Arc<egui::Galley> {
    layout_text(ui, text, width, false)
}

/// One-line links for compact tables; actions retain the entire URI when elided.
pub fn show_truncated(ui: &mut egui::Ui, text: &str) {
    let galley = layout_text(ui, text, ui.available_width(), true);
    show(ui, text, galley);
}

fn layout_text(ui: &egui::Ui, text: &str, width: f32, truncated: bool) -> Arc<egui::Galley> {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let normal = egui::TextFormat {
        font_id: font.clone(),
        color: ui.visuals().text_color(),
        ..Default::default()
    };
    let linked = egui::TextFormat {
        font_id: font,
        color: ui.visuals().hyperlink_color,
        underline: egui::Stroke::new(1.0_f32, ui.visuals().hyperlink_color),
        ..Default::default()
    };
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    if truncated {
        job.wrap.max_rows = 1;
    }
    let mut end = 0;
    for link in links(text) {
        job.append(&text[end..link.bytes.start], 0.0, normal.clone());
        job.append(&text[link.bytes.clone()], 0.0, linked.clone());
        end = link.bytes.end;
    }
    job.append(&text[end..], 0.0, normal);
    ui.fonts(|fonts| fonts.layout_job(job))
}
pub fn show(ui: &mut egui::Ui, text: &str, galley: Arc<egui::Galley>) {
    let found = links(text);
    let response = ui
        .add(egui::Label::new(galley.clone()).wrap())
        .on_hover_text(text);
    if found.is_empty() {
        response.context_menu(|ui| {
            if ui.button("Копировать").clicked() {
                ui.ctx().copy_text(text.to_owned());
                ui.close_menu();
            }
        });
        return;
    }
    for (index, link) in found.iter().enumerate() {
        let mut offset = 0;
        for (row_index, row) in galley.rows.iter().enumerate() {
            let from = link.chars.start.saturating_sub(offset);
            let to = link.chars.end.saturating_sub(offset).min(row.glyphs.len());
            if from < to {
                let rect = egui::Rect::from_min_max(
                    egui::pos2(row.glyphs[from].pos.x, row.rect.top()),
                    egui::pos2(row.glyphs[to - 1].max_x(), row.rect.bottom()),
                )
                .translate(response.rect.min.to_vec2());
                let target = &text[link.bytes.clone()];
                let hit = ui
                    .interact(
                        rect,
                        ui.id().with(("comment-link", index, row_index)),
                        egui::Sense::click(),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(target);
                if hit.clicked() {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(target));
                }
                hit.context_menu(|ui| {
                    if ui.button("Скопировать ссылку").clicked() {
                        ui.ctx().copy_text(target.to_owned());
                        ui.close_menu();
                    }
                    let open_label = if target
                        .get(..7)
                        .is_some_and(|s| s.eq_ignore_ascii_case("http://"))
                        || target
                            .get(..8)
                            .is_some_and(|s| s.eq_ignore_ascii_case("https://"))
                    {
                        "Открыть в браузере по умолчанию"
                    } else {
                        "Открыть в приложении по умолчанию"
                    };
                    if ui.button(open_label).clicked() {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(target));
                        ui.close_menu();
                    }
                    if ui.button("Отмена").clicked() {
                        ui.close_menu();
                    }
                });
            }
            offset += row.char_count_including_newline();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_links_inside_unicode_text_and_keeps_balanced_url_punctuation() {
        let text = "Описание (https://example.org/wiki/Test_(demo)). Ещё HTTPS://example.net/a?q=1&b=2,\nftp://example.org javascript:alert(1) https://";
        let parsed = links(text);
        assert_eq!(
            parsed
                .iter()
                .map(|l| &text[l.bytes.clone()])
                .collect::<Vec<_>>(),
            [
                "https://example.org/wiki/Test_(demo)",
                "HTTPS://example.net/a?q=1&b=2"
            ]
        );
        for link in parsed {
            assert_eq!(
                text.chars()
                    .skip(link.chars.start)
                    .take(link.chars.len())
                    .collect::<String>(),
                text[link.bytes]
            );
        }
    }
    #[test]
    fn wrapped_link_left_click_and_context_actions_target_exact_url() {
        for truncated in [false, true] {
            for target in ["https://example.org/path/very-long-name?q=1&b=2", "magnet:?xt=urn:btih:0123456789abcdef0123456789abcdef01234567&dn=Example&tr=udp%3A%2F%2Ftracker.example.org%3A6969", "udp://tracker.example.org:6969/announce"] {
        let open_label=if target.starts_with("https:") { "Открыть в браузере по умолчанию" } else { "Открыть в приложении по умолчанию" };
        for action in [
            "left",
            "Скопировать ссылку",
            open_label,
            "Отмена",
        ] {
            let ctx = egui::Context::default();
            let text = if truncated { format!("{target} конец") } else { format!("Описание: {target} конец") };
            let mut point = egui::Pos2::ZERO;
            let mut menu_point = egui::Pos2::ZERO;
            let mut opened = Vec::new();
            let mut copied = Vec::new();
            for frame in 0..6 {
                let mut input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(700.0, 500.0),
                    )),
                    ..Default::default()
                };
                if matches!(frame, 1 | 2 | 4 | 5) && !(action == "left" && frame >= 4) {
                    let pos = if frame < 4 { point } else { menu_point };
                    input.events = vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: if action != "left" && frame < 4 {
                                egui::PointerButton::Secondary
                            } else {
                                egui::PointerButton::Primary
                            },
                            pressed: frame == 1 || frame == 4,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ];
                }
                let output = ctx.run(input, |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let galley = layout_text(ui, &text, if truncated { 240.0 } else { 140.0 }, truncated);
                        if frame == 0 {
                            let row = &galley.rows[if truncated { 0 } else { 1 }];
                            let glyph = 1;
                            point = ui.cursor().min
                                + egui::vec2(row.glyphs[glyph].pos.x + 1.0, row.rect.center().y);
                        }
                        show(ui, &text, galley);
                    });
                });
                for command in &output.platform_output.commands {
                    match command {
                        egui::OutputCommand::OpenUrl(url) => opened.push(url.url.clone()),
                        egui::OutputCommand::CopyText(text) => copied.push(text.clone()),
                        _ => {}
                    }
                }
                if action != "left" && frame == 3 {
                    menu_point = output
                        .shapes
                        .iter()
                        .find_map(|s| match &s.shape {
                            egui::Shape::Text(t) if t.galley.text() == action => {
                                Some(t.pos + t.galley.size() / 2.0)
                            }
                            _ => None,
                        })
                        .expect("context menu item");
                }
            }
            assert_eq!(
                opened,
                if action == "left" || action == open_label
                {
                    vec![target.to_owned()]
                } else {
                    vec![]
                },
                "{action}"
            );
            assert_eq!(
                copied,
                if action == "Скопировать ссылку" {
                    vec![target.to_owned()]
                } else {
                    vec![]
                },
                "{action}"
            );
        }
        }
        }
    }
}
