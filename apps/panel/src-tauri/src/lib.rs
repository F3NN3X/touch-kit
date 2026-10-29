//! Touch panel app: touch-core on the Rust side, events into each panel's webview over a Channel.

mod bridge;
mod calibrate;
mod config;
mod driver;
mod pairing;
mod panels;
mod reload;
mod settings;
mod testing;
#[cfg(windows)]
mod win;
mod window;

use bridge::UiMsg;
use panels::Panels;
use std::sync::Mutex;
use tauri::ipc::Channel;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, RunEvent, State, Webview};

/// A panel window asks for its touch stream; which panel it is follows from its label.
#[tauri::command]
fn subscribe(webview: Webview, panels: State<'_, Panels>, channel: Channel<UiMsg>) {
    panels.subscribe(webview.label(), channel);
}

#[tauri::command]
fn ui_config(cfg: State<'_, Mutex<config::UiConfig>>) -> config::UiConfig {
    cfg.lock().unwrap().clone()
}

/// A gesture action of type `log` or `emit`, run by the webview.
#[tauri::command]
fn log_action(text: String) {
    log::info!("action {text}");
}

/// An uncaught error or failed render in the webview, written to the log.
#[tauri::command]
fn webview_error(text: String) {
    log::error!("webview: {text}");
}

/// The webview's periodic latency summary, written to the log.
#[tauri::command]
fn latency_report(report: String) {
    log::info!("latency {report}");
}

fn tray(app: &tauri::App) -> tauri::Result<()> {
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let reload = MenuItem::with_id(app, "reload", "Reload config", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings, &reload, &quit])?;
    // Left click opens settings, right click the menu (Linux: menu only).
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Touch Kit")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                settings::open_async(tray.app_handle());
            }
        })
        .on_menu_event(|app, e| match e.id.as_ref() {
            "settings" => settings::open_async(app),
            "reload" => reload::request(),
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

pub fn run() {
    let app = tauri::Builder::default()
        // Default targets: stdout and `<app log dir>/Touch Panel.log`. touch-core's `tracing` events
        // arrive here too, through tracing's `log` feature. Latency lines alone are ~25 kB/h.
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .max_file_size(1_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(5))
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            subscribe,
            ui_config,
            latency_report,
            log_action,
            webview_error,
            settings::config_get,
            settings::config_set,
            settings::panels_status,
            settings::monitors,
            settings::identify,
            pairing::pair_start,
            pairing::pair_cancel,
            calibrate::calibrate_start,
            calibrate::calibrate_check,
            calibrate::calibrate_save,
            calibrate::calibrate_cancel,
            driver::driver_status,
            driver::driver_uninstall,
            driver::open_zadig,
        ])
        .setup(|app| {
            let path = app.path().app_config_dir()?.join("config.json");
            let loaded = config::load_or_init(&path);
            reload::log_issues(&path, &loaded.issues, "using the defaults");
            let cfg = loaded.config.unwrap_or_default();
            log::info!("config: {} ({} panels, dev_mode {})", path.display(), cfg.panels.len(), cfg.dev_mode);

            tray(app)?;
            app.manage(Mutex::new(config::UiConfig::new(&cfg, Vec::new())));
            app.manage(Panels::default());
            app.manage(pairing::Pairing::default());
            app.manage(settings::ConfigPath(path.clone()));
            app.manage(Mutex::new(cfg.clone()));
            let mut issues = loaded.issues.clone();
            issues.extend(app.state::<Panels>().start(app.handle(), &cfg));
            *app.state::<Mutex<config::UiConfig>>().lock().unwrap() = config::UiConfig::new(&cfg, issues);
            reload::spawn(app.handle().clone(), path, cfg);
            // `--settings` or `--settings=<page>`; also on first run.
            let arg = std::env::args().find(|a| a == "--settings" || a.starts_with("--settings="));
            if loaded.created || arg.is_some() {
                let page = arg.as_deref().and_then(|a| a.strip_prefix("--settings="));
                settings::open(app.handle(), page);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building the panel app");
    app.run(|_, event| {
        // With no panel window open (none configured, or none matched) the tray keeps the app alive;
        // only Quit (an explicit exit code) ends it.
        if let RunEvent::ExitRequested { code: None, api, .. } = event {
            api.prevent_exit();
        }
    });
}
