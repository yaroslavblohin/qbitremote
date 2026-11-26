#[cfg(any(windows, target_os = "macos"))]
#[path = "torrent_icons.rs"]
mod torrent_icons;

#[cfg(any(windows, target_os = "macos"))]
fn icon_picker(ui: &mut eframe::egui::Ui) -> &'static torrent_icons::FileIcon {
    use eframe::egui;
    let id = egui::Id::new("torrent-file-icon-resource");
    let icons = torrent_icons::ICONS;
    let saved = ui.ctx().data_mut(|data| data.get_persisted::<u32>(id));
    let mut selected = icons
        .iter()
        .find(|icon| Some(icon.resource) == saved)
        .unwrap_or(&icons[0]);
    ui.label("Значок файлов .torrent и .magnet");
    ui.horizontal_wrapped(|ui| {
        for icon in icons {
            let texture_id = id.with(icon.resource);
            let ctx = ui.ctx();
            let cached = ctx.data(|data| data.get_temp::<egui::TextureHandle>(texture_id));
            let texture = cached.unwrap_or_else(|| {
                // load_texture also reads the context, so do it outside a data lock.
                let rgba = torrent_icons::resize(&icon.decode(), 128);
                let color = egui::ColorImage::from_rgba_unmultiplied([128, 128], &rgba);
                let texture = ctx.load_texture(icon.title, color, egui::TextureOptions::LINEAR);
                ctx.data_mut(|data| data.insert_temp(texture_id, texture.clone()));
                texture
            });
            if ui
                .add(
                    egui::ImageButton::new(
                        egui::Image::new(&texture).fit_to_exact_size(egui::vec2(48.0, 48.0)),
                    )
                    .selected(selected.resource == icon.resource),
                )
                .on_hover_text(icon.title)
                .clicked()
            {
                selected = icon;
            }
        }
    });
    ui.ctx()
        .data_mut(|data| data.insert_persisted(id, selected.resource));
    selected
}

#[cfg(windows)]
pub fn ui(ui: &mut eframe::egui::Ui) {
    use eframe::egui;
    let preview = cfg!(feature = "ui-smoke")
        && std::env::var("QBIT_SMOKE_PAGE").as_deref() == Ok("associations");
    let section = eframe::egui::CollapsingHeader::new("Значок и ассоциации файлов")
        .default_open(preview)
        .show(ui, |ui| {
        let selected = icon_picker(ui).resource;
        ui.small("Выберите значок для .torrent и .magnet. После регистрации назначьте qBit Remote для этих файлов и magnet-ссылок в настройках Windows.");
        let status_id = egui::Id::new("torrent-file-icon-resource").with("status");
        if ui.button("Применить значок и зарегистрировать ассоциации").clicked() {
            let result = register(selected).map(|()| "Регистрация обновлена. Выберите qBit Remote в настройках Windows.".to_owned()).unwrap_or_else(|e| e);
            ui.ctx().data_mut(|d| d.insert_temp(status_id, result));
        }
        if ui.button("Приложения по умолчанию в Windows…").clicked() {
            ui.ctx().open_url(egui::OpenUrl::same_tab("ms-settings:defaultapps"));
        }
        if let Some(status) = ui.ctx().data_mut(|d| d.get_temp::<String>(status_id)) { ui.label(status); }
    });
    if preview {
        section.header_response.scroll_to_me(Some(egui::Align::Min));
    }
}

#[cfg(windows)]
fn register(resource: u32) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy();
    let command = format!("\"{exe}\" \"%1\"");
    let icon = format!("\"{exe}\",-{resource}");
    let values = [
        (
            "Software\\Classes\\Applications\\qbit-remote.exe\\DefaultIcon",
            "",
            icon.as_str(),
        ),
        (
            "Software\\Classes\\Applications\\qbit-remote.exe\\shell\\open\\command",
            "",
            command.as_str(),
        ),
        (
            "Software\\Classes\\Applications\\qbit-remote.exe\\SupportedTypes",
            ".torrent",
            "",
        ),
        (
            "Software\\Classes\\Applications\\qbit-remote.exe\\SupportedTypes",
            ".magnet",
            "",
        ),
        (
            "Software\\Classes\\qBitRemote.Torrent",
            "",
            "Торрент qBit Remote",
        ),
        (
            "Software\\Classes\\qBitRemote.Torrent\\DefaultIcon",
            "",
            icon.as_str(),
        ),
        (
            "Software\\Classes\\qBitRemote.Torrent\\shell\\open\\command",
            "",
            command.as_str(),
        ),
        (
            "Software\\Classes\\.torrent\\OpenWithProgids",
            "qBitRemote.Torrent",
            "",
        ),
        (
            "Software\\Classes\\qBitRemote.MagnetFile",
            "",
            "Magnet-файл qBit Remote",
        ),
        (
            "Software\\Classes\\qBitRemote.MagnetFile\\DefaultIcon",
            "",
            icon.as_str(),
        ),
        (
            "Software\\Classes\\qBitRemote.MagnetFile\\shell\\open\\command",
            "",
            command.as_str(),
        ),
        (
            "Software\\Classes\\.magnet\\OpenWithProgids",
            "qBitRemote.MagnetFile",
            "",
        ),
        (
            "Software\\Classes\\qBitRemote.Magnet",
            "",
            "Magnet qBit Remote",
        ),
        ("Software\\Classes\\qBitRemote.Magnet", "URL Protocol", ""),
        (
            "Software\\Classes\\qBitRemote.Magnet\\shell\\open\\command",
            "",
            command.as_str(),
        ),
        (
            "Software\\qBitRemote\\Capabilities",
            "ApplicationName",
            "qBit Remote",
        ),
        (
            "Software\\qBitRemote\\Capabilities",
            "ApplicationDescription",
            "Клиент удалённого qBittorrent",
        ),
        (
            "Software\\qBitRemote\\Capabilities\\FileAssociations",
            ".torrent",
            "qBitRemote.Torrent",
        ),
        (
            "Software\\qBitRemote\\Capabilities\\FileAssociations",
            ".magnet",
            "qBitRemote.MagnetFile",
        ),
        (
            "Software\\qBitRemote\\Capabilities\\URLAssociations",
            "magnet",
            "qBitRemote.Magnet",
        ),
        (
            "Software\\RegisteredApplications",
            "qBit Remote",
            "Software\\qBitRemote\\Capabilities",
        ),
    ];
    for (key, name, value) in values {
        let mut cmd = std::process::Command::new("reg.exe");
        cmd.creation_flags(0x08000000)
            .args(["add", &format!("HKCU\\{key}")]);
        if name.is_empty() {
            cmd.arg("/ve");
        } else {
            cmd.args(["/v", name]);
        }
        let out = cmd
            .args(["/t", "REG_SZ", "/d", value, "/f"])
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(format!(
                "Не удалось зарегистрировать ассоциации: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
    }
    unsafe {
        windows_sys::Win32::UI::Shell::SHChangeNotify(
            0x08000000,
            0,
            std::ptr::null(),
            std::ptr::null(),
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn ui(ui: &mut eframe::egui::Ui) {
    use eframe::egui;
    egui::CollapsingHeader::new("Значок и ассоциации файлов").show(ui, |ui| {
        let selected = icon_picker(ui).resource;
        crate::macos_settings::association_ui(ui, selected);
    });
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn ui(_ui: &mut eframe::egui::Ui) {}

#[cfg(all(test, any(windows, target_os = "macos")))]
mod tests {
    use super::*;
    use eframe::egui;

    #[test]
    fn opening_associations_loads_and_reuses_all_icon_previews() {
        let ctx = egui::Context::default();
        ctx.style_mut(|style| style.animation_time = 0.0);
        let frame = |events| {
            ctx.run(
                egui::RawInput {
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, ui);
                },
            )
        };
        let collapsed = frame(vec![]);
        let header = collapsed
            .shapes
            .iter()
            .find_map(|entry| match &entry.shape {
                egui::Shape::Text(text) if text.galley.text() == "Значок и ассоциации файлов" => {
                    Some(text.pos + text.galley.size() / 2.0)
                }
                _ => None,
            })
            .unwrap();
        let id = egui::Id::new("torrent-file-icon-resource");
        let cached_icons = || {
            ctx.data(|data| {
                torrent_icons::ICONS
                    .iter()
                    .filter_map(|icon| {
                        data.get_temp::<egui::TextureHandle>(id.with(icon.resource))
                            .map(|texture| texture.id())
                    })
                    .collect::<Vec<_>>()
            })
        };
        assert!(cached_icons().is_empty());
        let click = |pressed| {
            frame(vec![
                egui::Event::PointerMoved(header),
                egui::Event::PointerButton {
                    pos: header,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ])
        };
        let _ = click(true);
        let _ = click(false);
        let loaded = cached_icons();
        assert_eq!(loaded.len(), torrent_icons::ICONS.len());
        let _ = click(true);
        let _ = click(false);
        let _ = click(true);
        let reopened = click(false);
        assert_eq!(cached_icons(), loaded);
        assert!(reopened
            .textures_delta
            .set
            .iter()
            .all(|(id, _)| !loaded.contains(id)));
    }
}
