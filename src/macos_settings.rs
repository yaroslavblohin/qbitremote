use eframe::egui::{self, Context, Id, Ui};
use std::ffi::{c_char, c_int, c_void, CStr};

type Completion = extern "C" fn(*mut c_void, *const c_char);

extern "C" {
    fn qbit_macos_has_app_bundle() -> bool;
    fn qbit_macos_apply_associations(resource: u32, context: *mut c_void, result: Completion);
    fn qbit_macos_login_status() -> c_int;
    fn qbit_macos_set_login(enable: bool, context: *mut c_void, result: Completion);
    fn qbit_macos_open_login_settings();
}

pub fn has_app_bundle() -> bool {
    unsafe { qbit_macos_has_app_bundle() }
}

#[derive(Clone)]
enum Change {
    Pending,
    Finished(Result<(), String>),
}

fn start_change(ctx: &Context, id: Id) -> *mut c_void {
    ctx.data_mut(|data| data.insert_temp(id, Change::Pending));
    Box::into_raw(Box::new((ctx.clone(), id))).cast()
}

extern "C" fn finish_change(context: *mut c_void, error: *const c_char) {
    // The native operation calls this once, with an optional string valid for the call.
    let (ctx, id) = *unsafe { Box::from_raw(context.cast::<(Context, Id)>()) };
    let result = if error.is_null() {
        Ok(())
    } else {
        Err(unsafe { CStr::from_ptr(error) }
            .to_string_lossy()
            .into_owned())
    };
    ctx.data_mut(|data| data.insert_temp(id, Change::Finished(result)));
    ctx.request_repaint();
}

fn state(ui: &Ui, id: Id) -> Option<Change> {
    ui.ctx().data(|data| data.get_temp::<Change>(id))
}

fn show_change(ui: &mut Ui, id: Id, success: &str) {
    match state(ui, id) {
        Some(Change::Pending) => {
            ui.spinner();
        }
        Some(Change::Finished(Ok(()))) => {
            ui.small(success);
        }
        Some(Change::Finished(Err(error))) => {
            ui.colored_label(crate::theme::palette(ui.ctx()).red, error);
        }
        None => {}
    }
}

pub fn login_ui(ui: &mut Ui) {
    let id = Id::new("macos-login-change");
    let status = unsafe { qbit_macos_login_status() };
    let mut enabled = matches!(status, 1..=3);
    if ui
        .add_enabled(
            status != 4,
            egui::Checkbox::new(&mut enabled, "Запускать при входе в macOS"),
        )
        .changed()
    {
        unsafe { qbit_macos_set_login(enabled, start_change(ui.ctx(), id), finish_change) };
    }
    match status {
        2 => {
            ui.small("Разрешите запуск qBit Remote в системных настройках.");
            if ui.button("Объекты входа…").clicked() {
                unsafe { qbit_macos_open_login_settings() };
            }
        }
        3 => {
            ui.small("После переноса приложения выключите и снова включите автозапуск.");
        }
        4 => {
            ui.small("Для автозапуска откройте qBit Remote.app.");
        }
        _ => {}
    }
    show_change(ui, id, "Настройка автозапуска обновлена.");
}

pub fn association_ui(ui: &mut Ui, resource: u32) {
    if !has_app_bundle() {
        ui.small("Для назначения обработчика откройте qBit Remote.app.");
        return;
    }
    let id = Id::new("macos-file-associations");
    let pending = matches!(state(ui, id), Some(Change::Pending));
    ui.small("Применяет значок в Finder для .torrent и .magnet и назначает qBit Remote для файлов и magnet-ссылок.");
    if ui
        .add_enabled(
            !pending,
            egui::Button::new("Применить значок и зарегистрировать ассоциации"),
        )
        .clicked()
    {
        unsafe {
            qbit_macos_apply_associations(resource, start_change(ui.ctx(), id), finish_change)
        };
    }
    show_change(ui, id, "Значок и ассоциации применены.");
}
