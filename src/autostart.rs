use std::{ffi::OsStr, io, os::windows::ffi::OsStrExt, ptr};
use windows_sys::Win32::{
    Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
    System::Registry::{
        RegDeleteKeyValueW, RegGetValueW, RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ,
    },
};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const NAME: &str = "qBit Remote";
pub const VISIBLE_ARG: &str = "--autostart";
pub const HIDDEN_ARG: &str = "--autostart-hidden";

#[derive(Clone, Copy)]
pub struct Options {
    enabled: bool,
    hidden: bool,
}

fn wide(value: impl AsRef<OsStr>) -> Vec<u16> {
    value.as_ref().encode_wide().chain(Some(0)).collect()
}

impl Options {
    pub fn load() -> io::Result<Self> {
        let key = wide(RUN_KEY);
        let name = wide(NAME);
        let mut bytes = 0;
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut bytes,
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(Self {
                enabled: false,
                hidden: false,
            });
        }
        result(status)?;
        let mut command = vec![0u16; (bytes as usize).div_ceil(2)];
        result(unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                RRF_RT_REG_SZ,
                ptr::null_mut(),
                command.as_mut_ptr().cast(),
                &mut bytes,
            )
        })?;
        let command = String::from_utf16_lossy(&command);
        Ok(Self {
            enabled: true,
            hidden: command.trim_end_matches('\0').ends_with(HIDDEN_ARG),
        })
    }

    fn apply(self) -> io::Result<()> {
        let key = wide(RUN_KEY);
        let name = wide(NAME);
        if !self.enabled {
            let status =
                unsafe { RegDeleteKeyValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr()) };
            return if status == ERROR_FILE_NOT_FOUND {
                Ok(())
            } else {
                result(status)
            };
        }
        let exe = std::env::current_exe()?;
        let flag = if self.hidden { HIDDEN_ARG } else { VISIBLE_ARG };
        let mut command = vec![b'"' as u16];
        command.extend(exe.as_os_str().encode_wide());
        command.extend(format!("\" {flag}").encode_utf16());
        command.push(0);
        result(unsafe {
            RegSetKeyValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                name.as_ptr(),
                REG_SZ,
                command.as_ptr().cast(),
                (command.len() * 2) as u32,
            )
        })
    }
}

fn result(status: u32) -> io::Result<()> {
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}

pub fn ui(ui: &mut eframe::egui::Ui) {
    use eframe::egui;
    let id = egui::Id::new("windows-autostart-error");
    let mut options = match Options::load() {
        Ok(options) => options,
        Err(error) => {
            ui.small(format!(
                "Не удалось прочитать настройку автозапуска: {error}"
            ));
            return;
        }
    };
    let enabled_changed = ui
        .checkbox(&mut options.enabled, "Запускать вместе с Windows")
        .changed();
    let hidden_changed = ui
        .add_enabled(
            options.enabled,
            egui::Checkbox::new(&mut options.hidden, "При автозапуске скрывать окно в трей"),
        )
        .changed();
    if enabled_changed || hidden_changed {
        let error = options.apply().err().map(|err| err.to_string());
        ui.ctx().data_mut(|data| data.insert_temp(id, error));
    }
    if let Some(error) = ui
        .ctx()
        .data(|data| data.get_temp::<Option<String>>(id))
        .flatten()
    {
        ui.small(format!("Не удалось изменить автозапуск: {error}"));
    }
}
