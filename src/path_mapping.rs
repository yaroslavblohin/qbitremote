use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Default, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct PathMapping {
    pub server: String,
    pub local: String,
}

pub fn settings_ui(ui: &mut eframe::egui::Ui, mappings: &mut Vec<PathMapping>) {
    use eframe::egui;
    egui::CollapsingHeader::new("Сопоставление папок сервера с этим компьютером")
        .default_open(cfg!(feature = "ui-smoke") && std::env::var("QBIT_SMOKE_PAGE").as_deref() == Ok("path-mappings"))
        .show(ui, |ui| {
        ui.label("Для открытия файлов и папок из вкладки «Файлы». Сетевой ресурс должен быть доступен в Windows.");
        ui.weak("Пример: /downloads → Z:\\Downloads или \\\\NAS\\Downloads. Правила относятся только к выбранному профилю и сохраняются автоматически.");
        let mut remove = None;
        for (index, mapping) in mappings.iter_mut().enumerate() {
            ui.push_id(index, |ui| {
                ui.horizontal(|ui| {
                    ui.label("На сервере");
                    ui.add(egui::TextEdit::singleline(&mut mapping.server).desired_width(180.0).hint_text("/downloads"));
                    if ui.small_button("Удалить").clicked() { remove = Some(index); }
                });
                ui.horizontal(|ui| {
                    ui.label("На клиенте");
                    ui.add(egui::TextEdit::singleline(&mut mapping.local).desired_width(280.0).hint_text("Z:\\Downloads"));
                    if ui.small_button("Обзор…").clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() { mapping.local = path.display().to_string(); }
                    }
                });
                if let Err(error) = mapping.validate() { ui.colored_label(crate::theme::palette(ui.ctx()).orange, error); }
                ui.add_space(5.0);
            });
        }
        if let Some(index) = remove { mappings.remove(index); }
        if ui.button("Добавить сопоставление").clicked() { mappings.push(PathMapping::default()); }
    });
}

fn server_parts(path: &str) -> Result<(bool, Vec<String>), String> {
    let path = path.replace('\\', "/");
    let windows = path.as_bytes().get(1) == Some(&b':') || path.starts_with("//");
    if !(path.starts_with('/') || (windows && path.as_bytes().get(2) == Some(&b'/'))) {
        return Err("Путь на сервере должен быть абсолютным".into());
    }
    let mut parts = Vec::new();
    for part in path.split('/').filter(|p| !p.is_empty() && *p != ".") {
        if part == ".." || part.contains('\0') {
            return Err("Путь на сервере содержит недопустимые компоненты".into());
        }
        parts.push(part.to_owned());
    }
    Ok((windows, parts))
}

fn safe_component(part: &str) -> bool {
    let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
    !part.is_empty()
        && part != "."
        && part != ".."
        && !part.ends_with(['.', ' '])
        && !part.chars().any(|c| c < ' ' || "<>:\"/\\|?*".contains(c))
        && !matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        && !(stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

impl PathMapping {
    pub fn validate(&self) -> Result<(), String> {
        server_parts(&self.server)?;
        let local = self.local.replace('/', "\\");
        let drive = local.as_bytes().get(1) == Some(&b':')
            && local
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphabetic)
            && local.as_bytes().get(2) == Some(&b'\\');
        let unc =
            local.starts_with("\\\\") && !local.starts_with("\\\\?") && !local.starts_with("\\\\.");
        let parts: Vec<_> = local.split('\\').filter(|p| !p.is_empty()).collect();
        if !(drive || unc)
            || (unc && parts.len() < 2)
            || !parts
                .iter()
                .skip(usize::from(drive))
                .all(|p| safe_component(p))
        {
            return Err(
                "На клиенте укажите абсолютный путь Windows: Z:\\Downloads или \\\\NAS\\Downloads"
                    .into(),
            );
        }
        Ok(())
    }
}

/// Resolve only whole path components, preferring the most specific mapping.
/// File API names are relative to the torrent's save_path (including root folder).
pub fn resolve(
    mappings: &[PathMapping],
    save_path: &str,
    relative: &str,
) -> Result<PathBuf, String> {
    let (windows, mut target) = server_parts(save_path)?;
    let relative = relative.replace('\\', "/");
    if relative.starts_with('/') {
        return Err("Ожидался относительный путь файла торрента".into());
    }
    for part in relative.split('/').filter(|p| !p.is_empty()) {
        if !safe_component(part) {
            return Err("Имя файла несовместимо с путями Windows".into());
        }
        target.push(part.to_owned());
    }
    let mut best = None;
    for mapping in mappings {
        let Ok((style, prefix)) = server_parts(&mapping.server) else {
            continue;
        };
        if style == windows
            && prefix.len() <= target.len()
            && prefix.iter().zip(&target).all(|(a, b)| {
                if windows {
                    a.eq_ignore_ascii_case(b)
                } else {
                    a == b
                }
            })
            && best.as_ref().is_none_or(|(_, count)| prefix.len() > *count)
        {
            best = Some((mapping, prefix.len()));
        }
    }
    let (mapping, count) = best
        .ok_or("Нет сопоставления этого пути. Добавьте его в настройках подключения к серверу.")?;
    mapping.validate()?;
    let mut result = PathBuf::from(mapping.local.replace('/', "\\"));
    for part in &target[count..] {
        if !safe_component(part) {
            return Err("Имя файла несовместимо с путями Windows".into());
        }
        result.push(part);
    }
    Ok(result)
}

#[derive(Clone, Copy)]
pub enum OpenMode {
    Exact,
    Folder,
    File,
}

pub fn torrent_path(
    mappings: &[PathMapping],
    torrent: &crate::model::Torrent,
) -> Result<PathBuf, String> {
    let path = torrent
        .extra
        .get("content_path")
        .and_then(serde_json::Value::as_str)
        .filter(|path| !path.is_empty())
        .unwrap_or(&torrent.save_path);
    resolve(mappings, path, "")
}

fn choose_target(
    path: PathBuf,
    mode: OpenMode,
    pick_file: impl FnOnce(&std::path::Path) -> Option<PathBuf>,
) -> Result<Option<PathBuf>, String> {
    let metadata = std::fs::metadata(&path)
        .map_err(|e| format!("Путь недоступен: {}\n{e}", path.display()))?;
    match mode {
        OpenMode::Folder if !metadata.is_dir() => path
            .parent()
            .map(|p| Some(p.to_owned()))
            .ok_or_else(|| "У файла нет родительской папки".into()),
        OpenMode::File if metadata.is_dir() => Ok(pick_file(&path)),
        _ => Ok(Some(path)),
    }
}

pub fn open_target(path: PathBuf, mode: OpenMode) -> Result<Option<PathBuf>, String> {
    let Some(path) = choose_target(path, mode, |directory| {
        rfd::FileDialog::new()
            .set_title("Открыть файл торрента")
            .set_directory(directory)
            .pick_file()
    })?
    else {
        return Ok(None);
    };
    open(&path)?;
    Ok(Some(path))
}

pub fn open(path: &std::path::Path) -> Result<(), String> {
    // Check existence before invoking the shell; no command interpreter is used.
    std::fs::metadata(path).map_err(|e| format!("Путь недоступен: {}\n{e}", path.display()))?;
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let verb: Vec<u16> = "open\0".encode_utf16().collect();
        let com = unsafe {
            windows_sys::Win32::System::Com::CoInitializeEx(
                std::ptr::null(),
                windows_sys::Win32::System::Com::COINIT_APARTMENTTHREADED as u32
                    | windows_sys::Win32::System::Com::COINIT_DISABLE_OLE1DDE as u32,
            )
        };
        if com < 0 {
            return Err(format!(
                "Не удалось инициализировать Windows Shell (HRESULT {com:#x})"
            ));
        }
        use windows_sys::Win32::UI::Shell::{
            ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SHELLEXECUTEINFOW,
        };
        // This worker has no message loop and exits after the call. Wait for Shell
        // activation/DDE to finish before dropping its COM apartment.
        let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;
        info.lpVerb = verb.as_ptr();
        info.lpFile = wide.as_ptr();
        info.nShow = windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let result = unsafe { ShellExecuteExW(&mut info) };
        // Capture GetLastError before CoUninitialize can overwrite it.
        let error = (result == 0).then(std::io::Error::last_os_error);
        if com >= 0 {
            unsafe {
                windows_sys::Win32::System::Com::CoUninitialize();
            }
        }
        if let Some(error) = error {
            return Err(format!("Не удалось открыть {}: {error}", path.display()));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    Err("Открытие доступно только в Windows".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    #[test]
    #[ignore = "launches Explorer; run scripts/test-shell-open.ps1 on a Windows desktop"]
    fn native_shell_open_from_short_lived_worker() {
        let local = std::env::var("QBIT_TEST_OPEN_PATH").expect("Explicit fixture path required");
        let mode = std::env::var("QBIT_TEST_OPEN_MODE").expect("Explicit entry point required");
        let server = std::env::var("QBIT_TEST_OPEN_SERVER").unwrap_or_else(|_| "/downloads".into());
        let relative = std::env::var("QBIT_TEST_OPEN_RELATIVE")
            .unwrap_or_else(|_| "Папка с пробелами & тест".into());
        let mappings = vec![mapping(&server, &local)];
        let expected = PathBuf::from(&local).join(&relative);
        let result = std::thread::spawn(move || {
            let path = if mode == "torrent" {
                let mut torrent = crate::model::Torrent {
                    save_path: server.clone(),
                    ..Default::default()
                };
                torrent.extra.insert(
                    "content_path".into(),
                    serde_json::json!(format!("{server}/{relative}")),
                );
                torrent_path(&mappings, &torrent)
            } else {
                assert_eq!(mode, "tree");
                resolve(&mappings, &server, &relative)
            }?;
            open_target(path, OpenMode::Folder)
        })
        .join()
        .unwrap()
        .unwrap();
        assert_eq!(result, Some(expected));
    }
    #[test]
    fn torrent_uses_content_path_instead_of_display_name_and_falls_back_to_save_path() {
        let maps = [mapping("/downloads", "Z:\\Downloads")];
        let mut torrent = crate::model::Torrent {
            save_path: "/downloads".into(),
            name: "Renamed display title".into(),
            ..Default::default()
        };
        torrent.extra.insert(
            "content_path".into(),
            serde_json::json!("/downloads/Pack/file.iso"),
        );
        assert_eq!(
            torrent_path(&maps, &torrent).unwrap(),
            PathBuf::from("Z:\\Downloads\\Pack\\file.iso")
        );
        torrent.extra.remove("content_path");
        assert_eq!(
            torrent_path(&maps, &torrent).unwrap(),
            PathBuf::from("Z:\\Downloads")
        );
    }
    #[test]
    fn file_and_folder_commands_choose_correct_target_and_allow_cancel() {
        let root = std::env::temp_dir().join(format!(
            "qbit-open-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let file = root.join("example.txt");
        std::fs::write(&file, "fixture").unwrap();
        assert_eq!(
            choose_target(file.clone(), OpenMode::File, |_| panic!(
                "Single file must open directly"
            ))
            .unwrap(),
            Some(file.clone())
        );
        assert_eq!(
            choose_target(file.clone(), OpenMode::Folder, |_| panic!(
                "No picker for folders"
            ))
            .unwrap(),
            Some(root.clone())
        );
        assert_eq!(
            choose_target(root.clone(), OpenMode::Folder, |_| panic!(
                "No picker for folders"
            ))
            .unwrap(),
            Some(root.clone())
        );
        assert_eq!(
            choose_target(root.clone(), OpenMode::File, |directory| {
                assert_eq!(directory, root);
                Some(file.clone())
            })
            .unwrap(),
            Some(file)
        );
        assert_eq!(
            choose_target(root.clone(), OpenMode::File, |_| None).unwrap(),
            None
        );
        std::fs::remove_file(root.join("example.txt")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
    fn mapping(server: &str, local: &str) -> PathMapping {
        PathMapping {
            server: server.into(),
            local: local.into(),
        }
    }
    #[test]
    fn longest_boundary_match_and_unicode() {
        let maps = [
            mapping("/data", "Z:\\"),
            mapping("/data/media/", "\\\\NAS\\Media"),
        ];
        assert_eq!(
            resolve(&maps, "/data/media", "Фильм/часть 1.mkv").unwrap(),
            PathBuf::from("\\\\NAS\\Media\\Фильм\\часть 1.mkv")
        );
        assert!(resolve(&maps, "/database", "a").is_err());
        assert!(resolve(&maps, "/Data", "a").is_err());
        assert_eq!(
            resolve(&maps, "/data/media", "").unwrap(),
            PathBuf::from("\\\\NAS\\Media")
        );
    }
    #[test]
    fn windows_server_and_root_mapping() {
        let maps = [mapping("D:/Downloads", "Z:\\")];
        assert_eq!(
            resolve(&maps, "d:\\downloads\\Games", "Pack\\a.bin").unwrap(),
            PathBuf::from("Z:\\Games\\Pack\\a.bin")
        );
        assert_eq!(
            resolve(&[mapping("/", "Z:\\")], "/downloads", "a").unwrap(),
            PathBuf::from("Z:\\downloads\\a")
        );
    }
    #[test]
    fn rejects_traversal_devices_and_relative_mounts() {
        let maps = [mapping("/data", "Z:\\")];
        for relative in [
            "../a",
            "/a",
            "C:/a",
            "a/../../b",
            "a\\..\\b",
            "a:stream",
            "NUL.txt",
            "COM1",
            "a.",
            "a ",
            "a\0b",
        ] {
            assert!(resolve(&maps, "/data", relative).is_err(), "{relative}");
        }
        for local in [
            "relative",
            "Z:relative",
            "\\\\?\\C:\\data",
            "\\\\.\\NUL",
            "Z:\\..\\data",
            "\\\\NAS",
        ] {
            assert!(mapping("/data", local).validate().is_err(), "{local}");
        }
        assert!(resolve(&maps, "/data/../other", "a").is_err());
        assert!(resolve(&[], "/data", "a").is_err());
    }
    #[test]
    fn inaccessible_path_reports_error_without_launch() {
        assert!(open(std::path::Path::new(
            "Z:\\qbit-remote-nonexistent-test\\missing"
        ))
        .is_err());
    }
}
