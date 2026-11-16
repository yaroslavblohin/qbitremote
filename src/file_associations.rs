#[cfg(windows)]
#[path = "torrent_icons.rs"]
mod torrent_icons;
#[cfg(windows)]
pub fn ui(ui: &mut eframe::egui::Ui) {
    use eframe::egui;
    let preview = cfg!(feature = "ui-smoke")
        && std::env::var("QBIT_SMOKE_PAGE").as_deref() == Ok("associations");
    let section = eframe::egui::CollapsingHeader::new("Значок и ассоциации файлов")
        .default_open(preview)
        .show(ui, |ui| {
        let id = egui::Id::new("torrent-file-icon-resource");
        let mut selected = ui.ctx().data_mut(|d| d.get_persisted::<u32>(id).unwrap_or(2));
        if !torrent_icons::ICONS.iter().any(|icon| icon.resource == selected) { selected = 2; }
        ui.label("Значок .torrent:");
        ui.horizontal_wrapped(|ui| {
            for icon in torrent_icons::ICONS {
                let texture_id = id.with(icon.resource);
                let texture = ui.ctx().data_mut(|d| d.get_temp::<egui::TextureHandle>(texture_id)).unwrap_or_else(|| {
                    let rgba = torrent_icons::resize(&icon.decode(), 128);
                    let t = ui.ctx().load_texture(icon.title, egui::ColorImage::from_rgba_unmultiplied([128, 128], &rgba), egui::TextureOptions::LINEAR);
                    ui.ctx().data_mut(|d| d.insert_temp(texture_id, t.clone()));
                    t
                });
                if ui.add(egui::ImageButton::new(egui::Image::new(&texture).fit_to_exact_size(egui::vec2(48.0, 48.0))).selected(selected == icon.resource)).on_hover_text(icon.title).clicked() { selected = icon.resource; }
            }
        });
        ui.ctx().data_mut(|d| d.insert_persisted(id, selected));
        ui.small("Применение регистрирует qBit Remote для .torrent и magnet-ссылок. Выберите его по умолчанию в настройках Windows.");
        let status_id = id.with("status");
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

#[cfg(not(windows))]
pub fn ui(_ui: &mut eframe::egui::Ui) {}
