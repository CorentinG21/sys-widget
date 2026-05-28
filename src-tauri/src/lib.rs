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
#[tauri::command]
fn restart_app(app: AppHandle) {
    app.restart();
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
            let script_path = app_handle
                .path()
                .resource_dir()
                .expect("resource_dir unavailable")
                .join("hardware")
                .join("read_temp.ps1");

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
