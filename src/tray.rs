use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    PostMessageW, SetForegroundWindow, ShowWindowAsync, SW_RESTORE, SW_SHOWNOACTIVATE, WM_CLOSE,
};

#[derive(Clone)]
pub struct WindowControl {
    hwnd: isize,
    quitting: Arc<AtomicBool>,
    restored: Arc<AtomicBool>,
}
impl WindowControl {
    pub fn quitting(&self) -> bool {
        self.quitting.load(Ordering::Acquire)
    }
    pub fn restore(&self) {
        if self.quitting() {
            return;
        }
        self.restored.store(true, Ordering::Release);
        // Never lock egui from a callback inside the Win32 message loop.
        unsafe {
            ShowWindowAsync(self.hwnd as _, SW_RESTORE);
            SetForegroundWindow(self.hwnd as _);
        }
    }
    pub fn take_restored(&self) -> bool {
        self.restored.swap(false, Ordering::AcqRel)
    }
    pub fn exit(&self) {
        self.quitting.store(true, Ordering::Release);
        // Wake hidden windows before posting close, so eframe saves and exits.
        unsafe {
            ShowWindowAsync(self.hwnd as _, SW_SHOWNOACTIVATE);
            PostMessageW(self.hwnd as _, WM_CLOSE, 0, 0);
        }
    }
    #[cfg(feature = "ui-smoke")]
    pub fn close_to_tray(&self) {
        unsafe {
            PostMessageW(self.hwnd as _, WM_CLOSE, 0, 0);
        }
    }
    #[cfg(feature = "ui-smoke")]
    pub fn visible(&self) -> bool {
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible(self.hwnd as _) != 0 }
    }
    #[cfg(feature = "ui-smoke")]
    pub fn minimize(&self) {
        unsafe {
            ShowWindowAsync(
                self.hwnd as _,
                windows_sys::Win32::UI::WindowsAndMessaging::SW_MINIMIZE,
            );
        }
    }
    #[cfg(feature = "ui-smoke")]
    pub fn minimized(&self) -> bool {
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::IsIconic(self.hwnd as _) != 0 }
    }
}

pub struct Tray {
    _icon: TrayIcon,
    pub control: WindowControl,
}
impl Tray {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        let RawWindowHandle::Win32(handle) =
            cc.window_handle().map_err(|e| e.to_string())?.as_raw()
        else {
            return Err("Не удалось получить HWND окна".into());
        };
        let control = WindowControl {
            hwnd: handle.hwnd.get(),
            quitting: Arc::new(AtomicBool::new(false)),
            restored: Arc::new(AtomicBool::new(false)),
        };
        let menu = Menu::new();
        let show = MenuItem::new("Открыть qBit Remote", true, None);
        let exit = MenuItem::new("Выход", true, None);
        menu.append_items(&[&show, &exit])
            .map_err(|e| e.to_string())?;
        let mut rgba = Vec::with_capacity(32 * 32 * 4);
        for y in 0..32 {
            for x in 0_i32..32 {
                let arrow = (14..18).contains(&x) && (7..22).contains(&y)
                    || (17..24).contains(&y) && (x - 16).abs() <= 24 - y;
                rgba.extend_from_slice(if arrow {
                    &[235, 240, 245, 255]
                } else {
                    &[42, 51, 61, 255]
                });
            }
        }
        let icon = Icon::from_rgba(rgba, 32, 32).map_err(|e| e.to_string())?;
        let tray = TrayIconBuilder::new()
            .with_tooltip("qBit Remote")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .build()
            .map_err(|e| e.to_string())?;
        let show_id = show.id().clone();
        let exit_id = exit.id().clone();
        let menu_control = control.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if event.id == show_id {
                menu_control.restore();
            }
            if event.id == exit_id {
                menu_control.exit();
            }
        }));
        let click_control = control.clone();
        TrayIconEvent::set_event_handler(Some(move |event| {
            if matches!(event, TrayIconEvent::DoubleClick { .. }) {
                click_control.restore();
            }
        }));
        Ok(Self {
            _icon: tray,
            control,
        })
    }
}
impl Drop for Tray {
    fn drop(&mut self) {
        MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
        TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
    }
}

pub fn should_hide(
    close_requested: bool,
    close_to_tray: bool,
    tray_available: bool,
    quitting: bool,
) -> bool {
    close_requested && close_to_tray && tray_available && !quitting
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_exit_always_wins_over_close_to_tray() {
        for close in [false, true] {
            for enabled in [false, true] {
                for available in [false, true] {
                    assert!(!should_hide(close, enabled, available, true));
                }
            }
        }
        assert!(should_hide(true, true, true, false));
        assert!(!should_hide(true, true, false, false));
        assert!(!should_hide(true, false, true, false));
    }
}
