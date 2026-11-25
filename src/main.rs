#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod activation;
mod add_torrent;
mod api;
mod app;
#[cfg(windows)]
mod autostart;
mod columns;
mod console;
mod country_flags;
#[cfg(windows)]
mod credentials;
#[cfg(target_os = "macos")]
#[path = "macos_credentials.rs"]
mod credentials;
mod file_associations;
mod file_icons;
mod graphs;
mod icon;
#[cfg(target_os = "macos")]
mod macos_app;
mod model;
mod path_mapping;
mod priority;
mod remote;
mod rss_rules;
mod selection;
mod server_space;
#[cfg(windows)]
mod single_instance;
#[cfg(all(feature = "ui-smoke", windows))]
mod smoke;
mod status_bar;
mod theme;
#[cfg(windows)]
mod tray;
#[cfg(target_os = "macos")]
#[path = "macos_tray.rs"]
mod tray;
mod ui_icons;

// RTSS's per-executable opt-out. Exported as DATA by build.rs, before OpenGL starts.
// https://github.com/beeradmoore/RTSSHooksCompatibility-sample
#[cfg(windows)]
#[no_mangle]
#[used]
pub static RTSSHooksCompatibility: u32 = 0;

fn main() -> eframe::Result {
    #[cfg(all(feature = "ui-smoke", windows))]
    if std::env::var_os("QBIT_IMPORT_BRANDING").is_some() {
        icon::import_branding();
        return Ok(());
    }

    #[cfg(all(feature = "ui-smoke", windows))]
    if std::env::var_os("QBIT_EXPORT_BRANDING").is_some() {
        icon::export_branding();
        return Ok(());
    }
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    #[cfg(windows)]
    let (autostart, hidden) = match args.as_slice() {
        [flag] if flag == autostart::VISIBLE_ARG => (true, false),
        [flag] if flag == autostart::HIDDEN_ARG => (true, true),
        _ => (false, false),
    };
    #[cfg(windows)]
    let args = if autostart { Vec::new() } else { args };
    let launch_files = match activation::files(args) {
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
    #[cfg(target_os = "macos")]
    macos_app::install();
    #[cfg(windows)]
    let _instance = match single_instance::InstanceGuard::acquire() {
        Ok(Some(guard)) => guard,
        Ok(None) => {
            if autostart {
                return Ok(());
            }
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
            #[cfg(all(feature = "ui-smoke", windows))]
            persistence_path: Some(std::path::PathBuf::from("dist/ui-smoke-state.ron")),
            viewport: eframe::egui::ViewportBuilder::default()
                .with_icon(icon::window())
                .with_inner_size([1200.0, 780.0])
                .with_min_inner_size([860.0, 580.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            let mut app = app::TabbedApp::new(cc);
            #[cfg(windows)]
            if hidden {
                app.start_hidden(&cc.egui_ctx);
            }
            app.open_files(launch_files);
            #[cfg(all(feature = "ui-smoke", windows))]
            let app = smoke::SmokeApp::new(app, &cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}
