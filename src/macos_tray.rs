use eframe::egui::{Context, ViewportCommand};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};
#[derive(Clone)]
pub struct WindowControl {
    ctx: Context,
    quitting: Arc<AtomicBool>,
}
impl WindowControl {
    pub fn quitting(&self) -> bool {
        self.quitting.load(Ordering::Acquire)
    }
    pub fn take_visibility(&self) -> Option<bool> {
        None
    }
    pub fn restore(&self) {
        self.ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        self.ctx
            .send_viewport_cmd(ViewportCommand::Minimized(false));
        self.ctx.send_viewport_cmd(ViewportCommand::Focus);
    }
    pub fn exit(&self) {
        self.quitting.store(true, Ordering::Release);
        self.restore();
        self.ctx.send_viewport_cmd(ViewportCommand::Close);
    }
}
pub struct Tray {
    _icon: TrayIcon,
    pub control: WindowControl,
}
impl Tray {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Result<Self, String> {
        let control = WindowControl {
            ctx: cc.egui_ctx.clone(),
            quitting: Arc::new(AtomicBool::new(false)),
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
            .build()
            .map_err(|e| e.to_string())?;
        let callback = control.clone();
        let show_id = show.id().clone();
        let exit_id = exit.id().clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if event.id == show_id {
                callback.restore();
            }
            if event.id == exit_id {
                callback.exit();
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
    }
}
pub fn should_hide(close: bool, enabled: bool, available: bool, quitting: bool) -> bool {
    close && enabled && available && !quitting
}
