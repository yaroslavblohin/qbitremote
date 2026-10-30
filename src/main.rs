#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod activation;
mod add_torrent;
mod api;
mod app;
mod columns;
#[cfg(windows)]
mod credentials;
mod icon;
mod model;
mod remote;
mod rss_rules;
mod selection;
mod server_space;
#[cfg(windows)]
mod single_instance;
#[cfg(feature = "ui-smoke")]
mod smoke;
mod theme;
#[cfg(windows)]
mod tray;

fn main() -> eframe::Result {
    let launch_files = match activation::files(std::env::args_os().skip(1)) {
        Ok(files) => files,
        Err(error) => {
            rfd::MessageDialog::new()
                .set_title("Открытие торрента")
                .set_description(error)
                .set_level(rfd::MessageLevel::Error)
                .show();
            return Ok(());
        }
    };
    #[cfg(windows)]
    let _instance = match single_instance::InstanceGuard::acquire() {
        Ok(Some(guard)) => guard,
        Ok(None) => {
            let args = launch_files
                .into_iter()
                .map(|p| p.into_os_string())
                .collect::<Vec<_>>();
            if let Err(error) = activation::forward(&args) {
                rfd::MessageDialog::new()
                    .set_title("Открытие торрента")
                    .set_description(error)
                    .set_level(rfd::MessageLevel::Error)
                    .show();
            }
            return Ok(());
        }
        Err(error) => {
            rfd::MessageDialog::new()
                .set_title("qBit Remote")
                .set_description(format!(
                    "Не удалось проверить запущенный экземпляр приложения: {error}"
                ))
                .set_level(rfd::MessageLevel::Error)
                .show();
            return Ok(());
        }
    };
    eframe::run_native(
        "qBit Remote",
        eframe::NativeOptions {
            #[cfg(feature = "ui-smoke")]
            persistence_path: Some(std::path::PathBuf::from("dist/ui-smoke-state.ron")),
            viewport: eframe::egui::ViewportBuilder::default()
                .with_icon(icon::window())
                .with_inner_size([1200.0, 780.0])
                .with_min_inner_size([860.0, 580.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            let mut app = app::ClientApp::new(cc);
            app.open_files(launch_files);
            #[cfg(feature = "ui-smoke")]
            let app = smoke::SmokeApp::new(app);
            Ok(Box::new(app))
        }),
    )
}
