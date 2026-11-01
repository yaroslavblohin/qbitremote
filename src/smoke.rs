//! Opt-in renderer smoke test. Never included in the normal build.
use crate::app::ClientApp;
use eframe::{egui, App};
use std::time::{Duration, Instant};

pub struct SmokeApp {
    app: ClientApp,
    started: Instant,
    requested: bool,
    context_click_stage: u8,
    lifecycle: Option<std::thread::JoinHandle<()>>,
}
impl SmokeApp {
    pub fn new(mut app: ClientApp) -> Self {
        app.smoke_fixture();
        let lifecycle = if std::env::var_os("QBIT_SMOKE_TRAY").is_some() {
            let control = app.smoke_tray_control();
            Some(std::thread::spawn(move || {
                let click = |button| {
                    control.smoke_click(button, tray_icon::MouseButtonState::Down);
                    control.smoke_click(button, tray_icon::MouseButtonState::Up);
                };
                let wait_for = |visible| {
                    let deadline = Instant::now() + Duration::from_secs(3);
                    while control.visible() != visible {
                        assert!(
                            Instant::now() < deadline,
                            "window visibility did not become {visible}"
                        );
                        std::thread::sleep(Duration::from_millis(20));
                    }
                };
                std::thread::sleep(Duration::from_secs(1));
                for cycle in 0..10 {
                    println!("Tray cycle {cycle}: closing");
                    control.close_to_tray();
                    wait_for(false);
                    std::thread::sleep(Duration::from_millis(100));
                    control.restore();
                    wait_for(true);
                    std::thread::sleep(Duration::from_millis(100));
                    click(tray_icon::MouseButton::Left);
                    wait_for(false);
                    std::thread::sleep(Duration::from_millis(80));
                    click(tray_icon::MouseButton::Right);
                    std::thread::sleep(Duration::from_millis(80));
                    assert!(!control.visible(), "right-click changed window visibility");
                    click(tray_icon::MouseButton::Left);
                    wait_for(true);
                    std::thread::sleep(Duration::from_millis(80));
                }
                click(tray_icon::MouseButton::Left);
                wait_for(false);
                control.smoke_double_click();
                control.smoke_click(
                    tray_icon::MouseButton::Left,
                    tray_icon::MouseButtonState::Up,
                );
                std::thread::sleep(Duration::from_millis(100));
                assert!(!control.visible(), "double-click toggled the window twice");
                click(tray_icon::MouseButton::Left);
                wait_for(true);
                control.minimize();
                let deadline = Instant::now() + Duration::from_secs(3);
                while !control.minimized() {
                    assert!(Instant::now() < deadline, "window did not minimize");
                    std::thread::sleep(Duration::from_millis(20));
                }
                click(tray_icon::MouseButton::Left);
                let deadline = Instant::now() + Duration::from_secs(3);
                while control.minimized() {
                    assert!(
                        Instant::now() < deadline,
                        "left-click did not restore minimized window"
                    );
                    std::thread::sleep(Duration::from_millis(20));
                }
                println!("Tray clicks: left toggles, right preserves visibility, double-click suppressed, minimized restored");
                let exit_state = std::env::var("QBIT_SMOKE_TRAY").unwrap();
                if exit_state == "minimized" {
                    control.minimize();
                    let deadline = Instant::now() + Duration::from_secs(3);
                    while !control.minimized() {
                        assert!(Instant::now() < deadline, "window did not minimize");
                        std::thread::sleep(Duration::from_millis(20));
                    }
                } else if exit_state != "visible" {
                    control.close_to_tray();
                    wait_for(false);
                }
                std::thread::sleep(Duration::from_secs(1));
                println!("10 hide/restore cycles passed; exit state: {exit_state}");
                control.exit();
            }))
        } else {
            None
        };
        Self {
            app,
            started: Instant::now(),
            requested: false,
            context_click_stage: 0,
            lifecycle,
        }
    }
}
impl App for SmokeApp {
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        let page = std::env::var("QBIT_SMOKE_PAGE").unwrap_or_default();
        if !matches!(page.as_str(), "actions" | "actions-bottom") {
            return;
        }
        if let Some(pos) =
            ctx.data(|data| data.get_temp::<egui::Pos2>(egui::Id::new("smoke-torrent-row")))
        {
            if self.context_click_stage < 2 {
                input.events.push(egui::Event::PointerMoved(pos));
                input.events.push(egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Secondary,
                    pressed: self.context_click_stage == 0,
                    modifiers: egui::Modifiers::NONE,
                });
            } else if page == "actions-bottom" && self.context_click_stage == 2 {
                input
                    .events
                    .push(egui::Event::PointerMoved(pos + egui::vec2(40.0, 100.0)));
                input.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -850.0),
                    modifiers: egui::Modifiers::NONE,
                });
            } else {
                return;
            }
            self.context_click_stage += 1;
        }
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // Verify eframe invokes normal persistence, without changing user settings.
        if self.lifecycle.is_some() {
            println!("Tray lifecycle: save called");
        }
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(thread) = self.lifecycle.take() {
            thread.join().expect("tray lifecycle thread failed");
            println!("Tray lifecycle: normal exit");
        }
    }
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.update(ctx, frame);
        if ctx.input(|i| i.viewport().close_requested()) {
            println!("Smoke: native close requested");
        }
        if self.lifecycle.is_some() {
            return;
        }
        if !self.requested && self.started.elapsed() > Duration::from_secs(2) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.requested = true;
        }
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event {
                std::fs::create_dir_all("dist").unwrap();
                image::save_buffer(
                    "dist/ui-smoke.png",
                    image.as_raw(),
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        assert!(
            self.started.elapsed() < Duration::from_secs(20),
            "renderer screenshot timed out"
        );
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
