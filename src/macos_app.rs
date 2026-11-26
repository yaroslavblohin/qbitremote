use crate::add_torrent::Source;
use eframe::egui::{Context, ViewportCommand};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::{
    cell::RefCell,
    ffi::{c_char, c_void, CStr, OsStr, OsString},
    os::unix::ffi::OsStrExt,
};

#[derive(Default)]
struct Activation {
    ctx: Option<Context>,
    requests: Vec<(OsString, bool)>,
}

thread_local! {
    static ACTIVATION: RefCell<Activation> = RefCell::new(Activation::default());
}

extern "C" {
    fn qbit_macos_install();
    fn qbit_macos_set_window(view: *mut c_void);
    fn qbit_macos_show_window();
}

pub fn install() {
    // AppKit and all the callbacks below run on the main thread.
    unsafe { qbit_macos_install() };
}

pub fn set_window(cc: &eframe::CreationContext<'_>) {
    if let Ok(window) = cc.window_handle() {
        if let RawWindowHandle::AppKit(handle) = window.as_raw() {
            // AppKit keeps a weak reference to the NSWindow that owns this live NSView.
            unsafe { qbit_macos_set_window(handle.ns_view.as_ptr()) };
        }
    }
    ACTIVATION.with_borrow_mut(|state| state.ctx = Some(cc.egui_ctx.clone()));
}

pub fn show_window(ctx: &Context) {
    // Show the native window before requesting a repaint: it may have been ordered out.
    unsafe { qbit_macos_show_window() };
    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(ViewportCommand::Focus);
}

#[no_mangle]
extern "C" fn qbit_macos_reopen() {
    if let Some(ctx) = ACTIVATION.with_borrow(|state| state.ctx.clone()) {
        show_window(&ctx);
    }
}

#[no_mangle]
extern "C" fn qbit_macos_open(value: *const c_char, is_file: bool) {
    if value.is_null() {
        return;
    }
    // The caller owns the string for the duration of the callback.
    let value = unsafe { CStr::from_ptr(value) };
    let value = OsStr::from_bytes(value.to_bytes()).to_owned();
    ACTIVATION.with_borrow_mut(|state| state.requests.push((value, is_file)));
    qbit_macos_reopen();
}

pub fn take_open_requests() -> impl Iterator<Item = Result<Source, String>> {
    ACTIVATION
        .with_borrow_mut(|state| std::mem::take(&mut state.requests))
        .into_iter()
        .map(|(value, is_file)| {
            if is_file {
                return crate::activation::file_request(value.into());
            }
            let url = value.into_string().map_err(|_| "Недопустимая ссылка")?;
            crate::activation::magnet_link(&url)
                .ok_or_else(|| "Поддерживаются только magnet-ссылки".into())
        })
}
