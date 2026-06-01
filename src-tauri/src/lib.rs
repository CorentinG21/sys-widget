mod lhm;
mod models;
mod monitor;
mod startup;

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::time;

// Tauri manages its own tokio runtime; we import tokio::time directly
// because lib.rs uses interval_at (which needs tokio::time::Instant).
// Only the "time" feature is needed — "full" in Cargo.toml is kept for
// compatibility but reduced to ["time"] now.

use lhm::LhmProcess;
use models::MetricsPayload;
use monitor::Monitor;

// ─── Commands ────────────────────────────────────────────────────────────────

/// Update the metrics polling interval at runtime.
/// `ms` is clamped to [500, 10_000] ms.
#[tauri::command]
fn set_poll_interval(state: State<Arc<AtomicU64>>, ms: u64) {
    let clamped = ms.clamp(500, 10_000);
    state.store(clamped, Ordering::Relaxed);
}

#[tauri::command]
fn restart_app(app: AppHandle) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).spawn();
    }
    app.exit(0);
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Returns true if the startup task is currently registered.
#[tauri::command]
fn startup_is_registered() -> bool {
    startup::is_registered()
}

/// Toggle the startup task. Returns the new state (true = enabled).
#[tauri::command]
fn startup_toggle() -> bool {
    // Use the path of the currently-running exe — always correct regardless
    // of install location (currentUser NSIS, dev build, etc.).
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[startup] current_exe() failed: {e}");
            return false;
        }
    };
    startup::toggle(exe.to_string_lossy().as_ref())
}

#[derive(Serialize, Clone)]
pub struct UpdateInfo {
    pub version: String,
}

/// Check for updates and emit "update-available" if one is found.
/// Called automatically 30 s after start, and on demand from the frontend.
async fn check_updates(app: AppHandle) {
    use tauri_plugin_updater::UpdaterExt;

    match app.updater() {
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => {
                let version = update.version.clone();
                eprintln!("[updater] update available: {version}");
                if let Err(e) = app.emit("update-available", UpdateInfo { version }) {
                    eprintln!("[updater] emit error: {e}");
                }
            }
            Ok(None) => eprintln!("[updater] up to date"),
            Err(e) => eprintln!("[updater] check error: {e}"),
        },
        Err(e) => eprintln!("[updater] updater unavailable: {e}"),
    }
}

/// Trigger a manual update check from the frontend.
/// Returns Some(version) if an update is available, None if up to date.
/// Returns an Err string on network/config failure so the frontend can show feedback.
#[tauri::command]
async fn check_update(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_updater::UpdaterExt;

    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => {
            let version = update.version.clone();
            eprintln!("[updater] update available: {version}");
            if let Err(e) = app.emit("update-available", UpdateInfo { version: version.clone() }) {
                eprintln!("[updater] emit error: {e}");
            }
            Ok(Some(version))
        }
        Ok(None) => {
            eprintln!("[updater] up to date");
            Ok(None)
        }
        Err(e) => {
            eprintln!("[updater] check error: {e}");
            Err(e.to_string())
        }
    }
}

/// Download and install the available update.
#[tauri::command]
async fn install_update(app: AppHandle) {
    use tauri_plugin_updater::UpdaterExt;

    if let Ok(updater) = app.updater() {
        if let Ok(Some(update)) = updater.check().await {
            if let Err(e) = update.download_and_install(|_, _| {}, || {}).await {
                eprintln!("[updater] install error: {e}");
            }
        }
    }
}

// ─── App setup ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            set_poll_interval,
            restart_app,
            quit_app,
            startup_is_registered,
            startup_toggle,
            check_update,
            install_update,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // ── LHM subprocess ──────────────────────────────────────────────
            let prod_path = app_handle
                .path()
                .resource_dir()
                .ok()
                .map(|p| p.join("hardware").join("read_temp.ps1"));

            let script_path = match prod_path.filter(|p| p.exists()) {
                Some(p) => p,
                None => std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("hardware")
                    .join("read_temp.ps1"),
            };
            eprintln!(
                "[lhm] script_path = {:?} (exists={})",
                script_path,
                script_path.exists()
            );

            let lhm: Arc<Mutex<Option<LhmProcess>>> = Arc::new(Mutex::new(
                LhmProcess::start(script_path)
                    .map_err(|e| eprintln!("[lhm] startup error: {e}"))
                    .ok(),
            ));
            let lhm_for_cleanup = Arc::clone(&lhm);

            // ── Poll interval state (default 2 s, changeable at runtime) ───
            let poll_ms = Arc::new(AtomicU64::new(2_000));
            app.manage(poll_ms.clone());

            // ── Metrics loop ────────────────────────────────────────────────
            let mut monitor = Monitor::new();
            let app_metrics = app_handle.clone();

            tauri::async_runtime::spawn(async move {
                // Initial delay before first tick
                time::sleep(Duration::from_millis(poll_ms.load(Ordering::Relaxed))).await;
                loop {

                    let (cpu_percent, ram, disks, network, top_cpu) = monitor.collect();
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
                        top_cpu,
                    };

                    if let Err(e) = app_metrics.emit("metrics-updated", &payload) {
                        eprintln!("[metrics] emit error: {e}");
                    }

                    // Sleep for the current interval before next tick
                    time::sleep(Duration::from_millis(poll_ms.load(Ordering::Relaxed))).await;
                }
            });

            // ── Update check: 30 s after start, then every hour ─────────────
            let app_updater = app_handle.clone();
            tauri::async_runtime::spawn(async move {
                time::sleep(Duration::from_secs(30)).await;
                check_updates(app_updater.clone()).await;
                let mut interval = time::interval(Duration::from_secs(3600));
                interval.tick().await; // immediate first tick — skip it
                loop {
                    interval.tick().await;
                    check_updates(app_updater.clone()).await;
                }
            });

            // ── Cleanup on window destroy ───────────────────────────────────
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
