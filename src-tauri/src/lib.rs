mod lhm;
mod models;
mod monitor;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::time::{self, Instant};

use lhm::LhmProcess;
use models::MetricsPayload;
use monitor::Monitor;

/// Tauri command: restart the application cleanly.
/// Spawns a new instance of the current executable then exits.
/// Using explicit spawn+exit instead of app.restart() to avoid a WebView2
/// teardown race that can kill the process before the new instance is launched.
#[tauri::command]
fn restart_app(app: AppHandle) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).spawn();
    }
    app.exit(0);
}

/// Tauri command: quit the application.
#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![restart_app, quit_app])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Locate the bundled read_temp.ps1.
            // In production: resource_dir() points to the bundle resources.
            // In dev mode:   resource_dir() → target/debug/ where hardware/ is
            //                NOT present, so fall back to CARGO_MANIFEST_DIR
            //                (= src-tauri/) which always has hardware/ next to it.
            let script_path = {
                let prod_path = app_handle
                    .path()
                    .resource_dir()
                    .ok()
                    .map(|p| p.join("hardware").join("read_temp.ps1"));

                match prod_path.filter(|p| p.exists()) {
                    Some(p) => p,
                    None => {
                        // Dev fallback: hardware/ lives next to Cargo.toml
                        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                            .join("hardware")
                            .join("read_temp.ps1")
                    }
                }
            };
            eprintln!("[lhm] script_path = {:?} (exists={})", script_path, script_path.exists());

            // Spawn LHM subprocess (non-fatal if unavailable).
            let lhm: Arc<Mutex<Option<LhmProcess>>> = Arc::new(Mutex::new(
                LhmProcess::start(script_path)
                    .map_err(|e| eprintln!("[lhm] startup error: {e}"))
                    .ok(),
            ));

            let lhm_for_cleanup = Arc::clone(&lhm);
            let mut monitor = Monitor::new();

            // 2-second metrics loop — use Tauri's built-in Tokio runtime.
            // interval_at delays the first tick by 2s so sysinfo has a full
            // measurement window before the first CPU% calculation.
            tauri::async_runtime::spawn(async move {
                let start = Instant::now() + Duration::from_secs(2);
                let mut ticker = time::interval_at(start, Duration::from_secs(2));
                loop {
                    ticker.tick().await;

                    let (cpu_percent, ram, disks, network) = monitor.collect();

                    // Read latest LHM data (CPU temp + GPU).
                    let lhm_data = lhm
                        .lock()
                        .ok()
                        .and_then(|g| g.as_ref().map(|l| l.latest()))
                        .unwrap_or_default();

                    let payload = MetricsPayload {
                        cpu: models::CpuMetrics {
                            percent: cpu_percent,
                            temp: lhm_data.cpu_temp,
                        },
                        gpu: lhm_data.gpu,
                        ram,
                        disks,
                        network,
                    };

                    if let Err(e) = app_handle.emit("metrics-updated", &payload) {
                        eprintln!("[metrics] emit error: {e}");
                    }
                }
            });

            // Clean up LHM subprocess when the main window is destroyed.
            if let Some(window) = app.get_webview_window("main") {
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::Destroyed = event {
                        if let Ok(mut guard) = lhm_for_cleanup.lock() {
                            if let Some(lhm) = guard.as_mut() {
                                lhm.cleanup();
                            }
                        }
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
