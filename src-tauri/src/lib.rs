//! Thin Tauri shell around `overhead-core`. All networking, geometry, scoring,
//! alerting and persistence live in the core crate; this file only wires the
//! engine to the window, the tray and desktop notifications.

use std::path::PathBuf;
use std::sync::Arc;

use overhead_core::alerts::AlertEvent;
use overhead_core::engine::{Emitter, EnrichmentEvent, Engine, EngineConfig, EngineHandle, Snapshot};
use overhead_core::settings::Settings;
use overhead_core::store::TodayData;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter as _, Manager, State, WebviewWindow};
use tauri_plugin_notification::NotificationExt;

struct TauriEmitter(AppHandle);

impl Emitter for TauriEmitter {
    fn snapshot(&self, s: &Snapshot) {
        let _ = self.0.emit("snapshot", s);
    }
    fn enrichment(&self, e: &EnrichmentEvent) {
        let _ = self.0.emit("enrichment", e);
    }
    fn alert(&self, a: &AlertEvent) {
        let _ = self.0.emit("alert", a);
        if a.notify {
            let _ = self.0.notification().builder().title(&a.title).body(&a.detail).show();
        }
    }
}

struct AppState {
    engine: EngineHandle,
}

fn apply_window_prefs(win: &WebviewWindow, s: &Settings) {
    let w = &s.window;
    let _ = win.set_always_on_top(w.always_on_top);
    let _ = win.set_decorations(!w.frameless && !w.kiosk);
    let _ = win.set_fullscreen(w.kiosk);
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.engine.settings()
}

#[tauri::command]
fn set_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) {
    if let Some(win) = app.get_webview_window("main") {
        apply_window_prefs(&win, &settings);
    }
    state.engine.set_settings(settings);
}

#[tauri::command]
fn pin(state: State<'_, AppState>, hex: Option<String>) {
    state.engine.pin(hex);
}

#[tauri::command]
fn select_plane(state: State<'_, AppState>, hex: String) {
    state.engine.select(hex);
}

#[tauri::command]
fn today(state: State<'_, AppState>) -> Result<TodayData, String> {
    state.engine.today()
}

#[tauri::command]
fn export_csv(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let csv = state.engine.export_csv()?;
    std::fs::write(path, csv).map_err(|e| e.to_string())
}

/// `--replay <file>` plays a recorded provider response instead of polling live.
fn replay_arg() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        if a == "--replay" {
            return args.next().map(PathBuf::from);
        }
        if let Some(v) = a.strip_prefix("--replay=") {
            return Some(PathBuf::from(v));
        }
    }
    None
}

/// Release builds have no console on Windows, so write panics to a file the user can find.
fn install_crash_log() {
    std::panic::set_hook(Box::new(|info| {
        let dir = std::env::var_os("LOCALAPPDATA")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Overhead");
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("crash.log"), format!("{info}\n"));
    }));
}

pub fn run() {
    install_crash_log();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let mut cfg = EngineConfig::new(data_dir);
            cfg.replay_file = replay_arg();
            let emitter = Arc::new(TauriEmitter(app.handle().clone()));
            // Engine::spawn uses tokio::spawn, which needs a runtime context; setup runs outside one.
            let engine = tauri::async_runtime::block_on(async move { Engine::spawn(cfg, emitter) });
            if let Some(win) = app.get_webview_window("main") {
                apply_window_prefs(&win, &engine.settings());
            }

            let show = MenuItem::with_id(app, "show", "Show Overhead", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let mut tray = TrayIconBuilder::new()
                .tooltip("Overhead")
                .menu(&menu)
                .on_menu_event(|app, ev| match ev.id.as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                });
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            if let Err(e) = tray.build(app) {
                tracing::warn!(error = %e, "tray icon unavailable");
            }

            app.manage(AppState { engine });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_settings, set_settings, pin, select_plane, today, export_csv])
        .run(tauri::generate_context!())
        .expect("error while running Overhead");
}
