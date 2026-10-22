//! Opt-in renderer smoke test. Never included in the normal build.
use crate::app::ClientApp;
use eframe::{egui, App};
use std::time::{Duration, Instant};

pub struct SmokeApp {
    app: ClientApp,
    started: Instant,
    requested: bool,
    lifecycle: Option<std::thread::JoinHandle<()>>,
}
impl SmokeApp {
    pub fn new(mut app: ClientApp) -> Self {
        app.smoke_fixture();
        let lifecycle = if std::env::var_os("QBIT_SMOKE_TRAY").is_some() {
            let control = app.smoke_tray_control();
            Some(std::thread::spawn(move || {
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
                }
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
            lifecycle,
        }
    }
}
impl App for SmokeApp {
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
