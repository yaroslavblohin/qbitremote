use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Arc,
};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    IsIconic, IsWindowVisible, PostMessageW, SetForegroundWindow, ShowWindowAsync, SW_HIDE,
    SW_RESTORE, SW_SHOWNOACTIVATE, WM_CLOSE,
};

#[derive(Clone)]
pub struct WindowControl {
    hwnd: isize,
    quitting: Arc<AtomicBool>,
    visibility: Arc<AtomicU8>,
    suppress_left_release: Arc<AtomicBool>,
}
impl WindowControl {
    pub fn quitting(&self) -> bool {
        self.quitting.load(Ordering::Acquire)
    }
    pub fn restore(&self) {
        if self.quitting() {
            return;
        }
        self.visibility.store(1, Ordering::Release);
        // Never lock egui from a callback inside the Win32 message loop.
        unsafe {
            ShowWindowAsync(self.hwnd as _, SW_RESTORE);
            SetForegroundWindow(self.hwnd as _);
        }
    }
    pub fn take_visibility(&self) -> Option<bool> {
        match self.visibility.swap(0, Ordering::AcqRel) {
            1 => Some(true),
            2 => Some(false),
            _ => None,
        }
    }
    pub fn toggle(&self) {
        if self.quitting() {
            return;
        }
        if unsafe { IsWindowVisible(self.hwnd as _) != 0 && IsIconic(self.hwnd as _) == 0 } {
            self.visibility.store(2, Ordering::Release);
            unsafe {
                ShowWindowAsync(self.hwnd as _, SW_HIDE);
            }
        } else {
            self.restore();
        }
    }
    fn handle_event(&self, event: &TrayIconEvent) {
        if toggle_on_event(event, &self.suppress_left_release) {
            self.toggle();
        }
    }
    #[cfg(feature = "ui-smoke")]
    pub fn smoke_click(&self, button: MouseButton, state: MouseButtonState) {
        self.handle_event(&click_event(button, state));
    }
    #[cfg(feature = "ui-smoke")]
    pub fn smoke_double_click(&self) {
        self.handle_event(&double_click_event(MouseButton::Left));
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

fn toggle_on_event(event: &TrayIconEvent, suppress_release: &AtomicBool) -> bool {
    match event {
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Down,
            ..
        } => {
            suppress_release.store(false, Ordering::Release);
            false
        }
        TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } => !suppress_release.swap(false, Ordering::AcqRel),
        TrayIconEvent::DoubleClick {
            button: MouseButton::Left,
            ..
        } => {
            suppress_release.store(true, Ordering::Release);
            false
        }
        _ => false,
    }
}
#[cfg(any(test, feature = "ui-smoke"))]
fn click_event(button: MouseButton, button_state: MouseButtonState) -> TrayIconEvent {
    TrayIconEvent::Click {
        id: Default::default(),
        position: Default::default(),
        rect: Default::default(),
        button,
        button_state,
    }
}
#[cfg(any(test, feature = "ui-smoke"))]
fn double_click_event(button: MouseButton) -> TrayIconEvent {
    TrayIconEvent::DoubleClick {
        id: Default::default(),
        position: Default::default(),
        rect: Default::default(),
        button,
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
            visibility: Arc::new(AtomicU8::new(0)),
            suppress_left_release: Arc::new(AtomicBool::new(false)),
        };
        let menu = Menu::new();
        let show = MenuItem::new("Открыть qBit Remote", true, None);
        let exit = MenuItem::new("Выход", true, None);
        menu.append_items(&[&show, &exit])
            .map_err(|e| e.to_string())?;
        let icon = Icon::from_rgba(crate::icon::rgba(32), 32, 32).map_err(|e| e.to_string())?;
        let tray = TrayIconBuilder::new()
            .with_tooltip("qBit Remote")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
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
            click_control.handle_event(&event);
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
    fn only_left_release_toggles_visibility() {
        let suppress = AtomicBool::new(false);
        for button in [MouseButton::Right, MouseButton::Middle] {
            for state in [MouseButtonState::Down, MouseButtonState::Up] {
                assert!(!toggle_on_event(&click_event(button, state), &suppress));
            }
            assert!(!toggle_on_event(&double_click_event(button), &suppress));
        }
        assert!(!toggle_on_event(
            &click_event(MouseButton::Left, MouseButtonState::Down),
            &suppress
        ));
        assert!(toggle_on_event(
            &click_event(MouseButton::Left, MouseButtonState::Up),
            &suppress
        ));
    }
    #[test]
    fn windows_double_click_sequence_toggles_once_and_next_click_still_works() {
        let suppress = AtomicBool::new(false);
        let events = [
            click_event(MouseButton::Left, MouseButtonState::Down),
            click_event(MouseButton::Left, MouseButtonState::Up),
            double_click_event(MouseButton::Left),
            click_event(MouseButton::Left, MouseButtonState::Up),
        ];
        assert_eq!(
            events
                .iter()
                .filter(|event| toggle_on_event(event, &suppress))
                .count(),
            1
        );
        assert!(!toggle_on_event(
            &click_event(MouseButton::Left, MouseButtonState::Down),
            &suppress
        ));
        assert!(toggle_on_event(
            &click_event(MouseButton::Left, MouseButtonState::Up),
            &suppress
        ));
        let _ = toggle_on_event(&double_click_event(MouseButton::Left), &suppress);
        assert!(!toggle_on_event(
            &click_event(MouseButton::Left, MouseButtonState::Down),
            &suppress
        ));
        assert!(toggle_on_event(
            &click_event(MouseButton::Left, MouseButtonState::Up),
            &suppress
        ));
    }
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
