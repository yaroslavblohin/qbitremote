#[cfg(windows)]
pub fn ui(ui: &mut eframe::egui::Ui) {
    use eframe::egui;
    ui.collapsing("Значок и ассоциации файлов", |ui| {
        let id = egui::Id::new("torrent-file-icon-qb");
        let mut qb = ui.ctx().data_mut(|d| d.get_persisted::<bool>(id).unwrap_or(false));
        ui.horizontal(|ui| {
            ui.label("Значок .torrent:");
            ui.selectable_value(&mut qb, false, "Магнит");
            ui.selectable_value(&mut qb, true, "Логотип qb");
        });
        ui.ctx().data_mut(|d| d.insert_persisted(id, qb));
        ui.small("Применение регистрирует qBit Remote для .torrent и magnet-ссылок. Выберите его по умолчанию в настройках Windows.");
        let status_id = id.with("status");
        if ui.button("Применить значок и зарегистрировать ассоциации").clicked() {
            let result = register(qb).map(|()| "Регистрация обновлена. Выберите qBit Remote в настройках Windows.".to_owned()).unwrap_or_else(|e| e);
            ui.ctx().data_mut(|d| d.insert_temp(status_id, result));
        }
        if ui.button("Приложения по умолчанию в Windows…").clicked() {
            ui.ctx().open_url(egui::OpenUrl::same_tab("ms-settings:defaultapps"));
        }
        if let Some(status) = ui.ctx().data_mut(|d| d.get_temp::<String>(status_id)) { ui.label(status); }
    });
}

#[cfg(windows)]
fn register(qb: bool) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy();
    let command = format!("\"{exe}\" \"%1\"");
    let icon = format!("\"{exe}\",-{}", if qb { 1 } else { 2 });
    let values = [
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
