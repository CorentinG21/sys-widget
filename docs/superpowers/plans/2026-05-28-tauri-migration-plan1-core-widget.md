# SysmonWidget v2 — Core Widget (Plan 1 of 2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Scaffold a Tauri v2 + Svelte 5 widget on branch `tauri-rewrite` that displays CPU/GPU/RAM/disks/network in a glassmorphism overlay, transparent and always-on-bottom, updating every 2 seconds.

**Architecture:** Rust backend owns a Tokio 2s timer. Each tick, `monitor.rs` collects metrics via `sysinfo` crate and `lhm.rs` reads temperatures/GPU from a persistent PowerShell subprocess (LibreHardwareMonitorLib.dll). Results are emitted as a Tauri event `metrics-updated` to the Svelte 5 frontend, which updates a `$state` object and re-renders reactive components.

**Tech Stack:** Tauri v2, Rust 1.77+, Svelte 5, TypeScript, Vite 5, sysinfo 0.32, tokio 1, serde 1, serde_json 1

**Scope:** Plan 1 = widget core (all metrics visible, glassmorphism UI). System integrations (startup, auto-update, GitHub Actions) are in Plan 2.

---

## File Map

| Path | Responsibility |
|------|----------------|
| `src-tauri/src/models.rs` | Serializable types shared between Rust and JS |
| `src-tauri/src/monitor.rs` | `Monitor` struct: CPU%, RAM, disks (10s cache), network delta |
| `src-tauri/src/lhm.rs` | `LhmProcess`: spawns PS subprocess, parses JSON stdout, thread-safe latest data |
| `src-tauri/src/main.rs` | App entry, 2s Tokio timer, emits `metrics-updated`, Tauri commands |
| `src-tauri/Cargo.toml` | Rust dependencies |
| `src-tauri/tauri.conf.json` | Window: frameless 320×600 transparent, no taskbar, bundle resources |
| `src-tauri/capabilities/default.json` | Tauri v2 permissions: window set-always-on-bottom |
| `src/lib/stores/metrics.svelte.ts` | `metrics` reactive `$state` + `listen('metrics-updated')` |
| `src/lib/utils/colors.ts` | Pure functions: `thresholdColor()`, `formatRate()`, `netColor()` |
| `src/lib/components/MetricRow.svelte` | Label + smooth progress bar + values (CPU/GPU/RAM) |
| `src/lib/components/DiskRow.svelte` | Single disk partition row |
| `src/lib/components/NetworkRow.svelte` | Upload + download with rate formatting |
| `src/App.svelte` | Root: assembles components, drag region, always-on-bottom |
| `src/app.css` | Global CSS reset + glassmorphism design tokens |
| `hardware/` | Kept from current project (read_temp.ps1 + LibreHardwareMonitorLib.dll) |

---

## Task 1: Git Branch + Tauri Scaffold

**Files:**
- Create: `src-tauri/` (scaffolded by Tauri CLI)
- Create: `src/` (scaffolded by Tauri CLI)
- Create: `package.json`, `vite.config.ts`, `svelte.config.js`, `tsconfig.json`

- [ ] **Step 1: Create branch**

```bash
git checkout main
git checkout -b tauri-rewrite
```

Expected: `Switched to a new branch 'tauri-rewrite'`

- [ ] **Step 2: Scaffold Tauri app with Svelte TypeScript template**

Run from `C:\Users\coren\sysmon-widget\`:
```powershell
npm create tauri-app@latest . -- --template svelte-ts --manager npm --force
```

When prompted:
- Package name: `sysmon-widget`
- Choose "svelte-ts" template
- Use `npm` as package manager

> Note: The `--force` flag overwrites if needed. The scaffold adds files without touching existing `.py` files.

- [ ] **Step 3: Install dependencies**

```powershell
npm install
```

Expected: `added NNN packages` with no errors.

- [ ] **Step 4: Verify it builds**

```powershell
cargo tauri dev
```

Expected: A small Tauri window with "Welcome to Tauri" opens. Close it.

- [ ] **Step 5: Initial commit**

```bash
git add src/ src-tauri/ package.json package-lock.json vite.config.ts svelte.config.js tsconfig.json
git commit -m "chore: scaffold Tauri v2 + Svelte 5 on tauri-rewrite branch"
```

---

## Task 2: Window Configuration

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`

- [ ] **Step 1: Configure window in tauri.conf.json**

Replace the `app.windows` section:

```json
{
  "$schema": "../node_modules/@tauri-apps/cli/schema.json",
  "productName": "SysmonWidget",
  "version": "2.0.0",
  "identifier": "com.corentingodon.sysmonwidget",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "SysmonWidget",
        "width": 320,
        "height": 600,
        "decorations": false,
        "transparent": true,
        "skipTaskbar": true,
        "shadow": false,
        "resizable": false,
        "x": 50,
        "y": 50
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"],
    "resources": ["hardware/**/*"]
  }
}
```

> ⚠️ `height: 600` is intentional — Windows enforces a ~120px minimum window height even with `decorations: false`. The window is 600px tall but fully transparent. Only the `.widget` div has a visible background. No dynamic resizing needed.

> ⚠️ `"resources": ["hardware/**/*"]` bundles `read_temp.ps1` + `LibreHardwareMonitorLib.dll` into the executable's resource directory, accessible at runtime via `app_handle.path().resource_dir()`.

- [ ] **Step 2: Add always-on-bottom permission in capabilities/default.json**

```json
{
  "$schema": "../node_modules/@tauri-apps/cli/schema/acl/schemas/capabilities/desktop-schema.json",
  "identifier": "default",
  "description": "Default permissions for SysmonWidget",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-set-always-on-bottom",
    "core:window:allow-start-dragging"
  ]
}
```

- [ ] **Step 3: Verify window config**

```powershell
cargo tauri dev
```

Expected: A transparent frameless window opens (may show blank white initially — that's fine, the transparent CSS comes in Task 4).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tauri.conf.json src-tauri/capabilities/default.json
git commit -m "feat: configure transparent frameless window 320x600"
```

---

## Task 3: Rust Dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Replace Cargo.toml dependencies section**

Open `src-tauri/Cargo.toml` and set:

```toml
[package]
name = "sysmon-widget"
version = "2.0.0"
description = "SysmonWidget v2 — Tauri rewrite"
authors = ["CorentinG21"]
edition = "2021"

[lib]
name = "sysmon_widget_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-store = "2"
tauri-plugin-process = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sysinfo = "0.32"
tokio = { version = "1", features = ["full"] }

[features]
default = ["custom-protocol"]
custom-protocol = ["tauri/custom-protocol"]
```

- [ ] **Step 2: Verify it compiles**

```powershell
cargo build --manifest-path src-tauri/Cargo.toml
```

Expected: `Finished dev` with no errors (may take a few minutes on first build).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "feat: add sysinfo, serde, tokio, tauri-plugin-process to Cargo.toml"
```

---

## Task 4: models.rs — Shared Types

**Files:**
- Create: `src-tauri/src/models.rs`

- [ ] **Step 1: Write tests first**

Add to `src-tauri/src/models.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricsPayload {
    pub cpu: CpuMetrics,
    pub gpu: Option<GpuMetrics>,
    pub ram: RamMetrics,
    pub disks: Vec<DiskInfo>,
    pub network: NetworkMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuMetrics {
    pub percent: f32,
    pub temp: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuMetrics {
    pub percent: f32,
    pub temp: Option<f32>,
    pub vram_used_gb: f64,
    pub vram_total_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RamMetrics {
    pub percent: f32,
    pub used_gb: f64,
    pub total_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub mount: String,
    pub percent: f32,
    pub used_gb: f64,
    pub total_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkMetrics {
    pub upload_mbps: f64,
    pub download_mbps: f64,
}

/// Data read from the LHM PowerShell subprocess.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct LhmData {
    pub cpu_temp: Option<f32>,
    pub gpu: Option<GpuMetrics>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_payload_serializes_to_json() {
        let payload = MetricsPayload {
            cpu: CpuMetrics { percent: 42.5, temp: Some(55.0) },
            gpu: None,
            ram: RamMetrics { percent: 60.0, used_gb: 9.6, total_gb: 16.0 },
            disks: vec![DiskInfo {
                mount: "C:\\".to_string(),
                percent: 50.0,
                used_gb: 250.0,
                total_gb: 500.0,
            }],
            network: NetworkMetrics { upload_mbps: 0.5, download_mbps: 1.2 },
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"percent\":42.5"));
        assert!(json.contains("\"temp\":55.0"));
        assert!(json.contains("\"mount\":\"C:\\\\\""));
    }

    #[test]
    fn lhm_data_deserializes_from_json() {
        let json = r#"{"cpu_temp": 52.3, "gpu": {"percent": 30.0, "temp": 61.0, "vram_used_gb": 2.0, "vram_total_gb": 8.0}}"#;
        let data: LhmData = serde_json::from_str(json).unwrap();
        assert_eq!(data.cpu_temp, Some(52.3));
        assert!(data.gpu.is_some());
    }

    #[test]
    fn lhm_data_handles_null_fields() {
        let json = r#"{"cpu_temp": null, "gpu": null}"#;
        let data: LhmData = serde_json::from_str(json).unwrap();
        assert!(data.cpu_temp.is_none());
        assert!(data.gpu.is_none());
    }
}
```

- [ ] **Step 2: Run tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml models
```

Expected:
```
test models::tests::lhm_data_deserializes_from_json ... ok
test models::tests::lhm_data_handles_null_fields ... ok
test models::tests::metrics_payload_serializes_to_json ... ok

test result: ok. 3 passed
```

- [ ] **Step 3: Declare module in main.rs**

In `src-tauri/src/main.rs`, add at the top (above `fn main`):
```rust
mod models;
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/models.rs src-tauri/src/main.rs
git commit -m "feat: add models.rs with MetricsPayload and LhmData types"
```

---

## Task 5: monitor.rs — Sysinfo Metrics

**Files:**
- Create: `src-tauri/src/monitor.rs`

- [ ] **Step 1: Write the failing test**

Create `src-tauri/src/monitor.rs`:

```rust
use std::time::Instant;
use sysinfo::{Disks, Networks, System};

use crate::models::{DiskInfo, NetworkMetrics, RamMetrics};

const DISK_CACHE_SECS: u64 = 10;
const BYTES_PER_GB: f64 = 1_073_741_824.0;
const BYTES_PER_MB: f64 = 1_048_576.0;

pub struct Monitor {
    sys: System,
    disks: Disks,
    networks: Networks,
    disk_cache: Vec<DiskInfo>,
    disk_cache_time: Instant,
    net_last_refresh: Instant,
}

impl Monitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        sys.refresh_memory();

        Monitor {
            sys,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            disk_cache: Vec::new(),
            disk_cache_time: Instant::now() - std::time::Duration::from_secs(DISK_CACHE_SECS + 1),
            net_last_refresh: Instant::now(),
        }
    }

    pub fn cpu_percent(&mut self) -> f32 {
        self.sys.refresh_cpu_usage();
        self.sys.global_cpu_usage()
    }

    pub fn ram(&mut self) -> RamMetrics {
        self.sys.refresh_memory();
        let used = self.sys.used_memory();
        let total = self.sys.total_memory();
        let percent = if total > 0 {
            (used as f32 / total as f32) * 100.0
        } else {
            0.0
        };
        RamMetrics {
            percent,
            used_gb: used as f64 / BYTES_PER_GB,
            total_gb: total as f64 / BYTES_PER_GB,
        }
    }

    pub fn disks(&mut self) -> Vec<DiskInfo> {
        if self.disk_cache_time.elapsed().as_secs() < DISK_CACHE_SECS {
            return self.disk_cache.clone();
        }

        self.disks.refresh();
        self.disk_cache = self
            .disks
            .list()
            .iter()
            .filter(|d| {
                let total = d.total_space();
                let fs = d.file_system().to_string_lossy();
                total > 0 && !fs.is_empty() && fs != "tmpfs" && fs != "devtmpfs"
            })
            .map(|d| {
                let total = d.total_space();
                let avail = d.available_space();
                let used = total.saturating_sub(avail);
                let percent = (used as f32 / total as f32) * 100.0;
                DiskInfo {
                    mount: d.mount_point().to_string_lossy().to_string(),
                    percent,
                    used_gb: used as f64 / BYTES_PER_GB,
                    total_gb: total as f64 / BYTES_PER_GB,
                }
            })
            .collect();

        self.disk_cache_time = Instant::now();
        self.disk_cache.clone()
    }

    pub fn network(&mut self) -> NetworkMetrics {
        let elapsed = self.net_last_refresh.elapsed().as_secs_f64().max(0.001);
        self.net_last_refresh = Instant::now();

        self.networks.refresh();
        let rx: u64 = self.networks.iter().map(|(_, n)| n.received()).sum();
        let tx: u64 = self.networks.iter().map(|(_, n)| n.transmitted()).sum();

        NetworkMetrics {
            upload_mbps: (tx as f64 / elapsed / BYTES_PER_MB).max(0.0),
            download_mbps: (rx as f64 / elapsed / BYTES_PER_MB).max(0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_percent_is_valid_range() {
        let mut mon = Monitor::new();
        // First call may return 0 (needs two calls to warm up sysinfo)
        let _ = mon.cpu_percent();
        std::thread::sleep(std::time::Duration::from_millis(200));
        let pct = mon.cpu_percent();
        assert!(pct >= 0.0 && pct <= 100.0, "CPU% out of range: {}", pct);
    }

    #[test]
    fn ram_used_lte_total() {
        let mut mon = Monitor::new();
        let ram = mon.ram();
        assert!(ram.total_gb > 0.0, "total RAM is 0");
        assert!(ram.used_gb <= ram.total_gb, "used > total: {} > {}", ram.used_gb, ram.total_gb);
        assert!(ram.percent >= 0.0 && ram.percent <= 100.0);
    }

    #[test]
    fn disks_non_empty_on_windows() {
        let mut mon = Monitor::new();
        let disks = mon.disks();
        // Any Windows machine has at least one disk partition
        assert!(!disks.is_empty(), "expected at least one disk, got none");
        for d in &disks {
            assert!(d.total_gb > 0.0);
            assert!(d.percent >= 0.0 && d.percent <= 100.0);
        }
    }

    #[test]
    fn disk_cache_returns_same_data_within_10s() {
        let mut mon = Monitor::new();
        let first = mon.disks();
        let second = mon.disks();
        assert_eq!(first.len(), second.len(), "disk list changed within cache window");
    }

    #[test]
    fn network_mbps_non_negative() {
        let mut mon = Monitor::new();
        let net = mon.network();
        assert!(net.upload_mbps >= 0.0);
        assert!(net.download_mbps >= 0.0);
    }
}
```

- [ ] **Step 2: Run tests to verify they pass**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml monitor
```

Expected:
```
test monitor::tests::cpu_percent_is_valid_range ... ok
test monitor::tests::disk_cache_returns_same_data_within_10s ... ok
test monitor::tests::disks_non_empty_on_windows ... ok
test monitor::tests::network_mbps_non_negative ... ok
test monitor::tests::ram_used_lte_total ... ok

test result: ok. 5 passed
```

- [ ] **Step 3: Declare module in main.rs**

Add to `src-tauri/src/main.rs`:
```rust
mod monitor;
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/monitor.rs src-tauri/src/main.rs
git commit -m "feat: monitor.rs — CPU/RAM/disks/network via sysinfo (5 tests passing)"
```

---

## Task 6: lhm.rs — PowerShell Subprocess for Temperatures

**Files:**
- Create: `src-tauri/src/lhm.rs`

> Context: `read_temp.ps1` (already in `hardware/`) runs as a persistent subprocess, emitting one JSON line per 2 seconds on stdout. Format: `{"cpu_temp": 52.3, "gpu": {"percent": 30.0, "temp": 61.0, "vram_used_gb": 2.1, "vram_total_gb": 8.0}}`. The `gpu` field is null if no supported GPU is found. This is unchanged from the Python version.

- [ ] **Step 1: Create lhm.rs**

```rust
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::models::LhmData;

pub struct LhmProcess {
    child: Option<Child>,
    latest: Arc<Mutex<LhmData>>,
}

impl LhmProcess {
    /// Spawns the PowerShell subprocess and starts a background reader thread.
    /// If the spawn fails (PowerShell not found, script missing), returns silently
    /// and `get()` will always return `LhmData::default()` (no temps/GPU).
    pub fn start(ps_script: &Path) -> Self {
        let latest = Arc::new(Mutex::new(LhmData::default()));

        let mut child = Command::new("powershell")
            .args([
                "-NonInteractive",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                ps_script.to_str().unwrap_or(""),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok();

        if let Some(ref mut proc) = child {
            if let Some(stdout) = proc.stdout.take() {
                let latest_ref = Arc::clone(&latest);
                thread::spawn(move || {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines().flatten() {
                        if let Ok(data) = serde_json::from_str::<LhmData>(&line) {
                            if let Ok(mut lock) = latest_ref.lock() {
                                *lock = data;
                            }
                        }
                    }
                });
            }
        }

        LhmProcess { child, latest }
    }

    /// Returns the most recently parsed LHM data (cloned for thread safety).
    pub fn get(&self) -> LhmData {
        self.latest
            .lock()
            .map(|l| l.clone())
            .unwrap_or_default()
    }

    /// Kills the PowerShell subprocess and waits for it to exit.
    /// Called on app shutdown before Tauri cleans up.
    pub fn cleanup(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for LhmProcess {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lhm_start_with_missing_script_does_not_panic() {
        // Passing a non-existent path — should silently fail and not crash
        let lhm = LhmProcess::start(Path::new("C:\\nonexistent\\read_temp.ps1"));
        let data = lhm.get();
        assert!(data.cpu_temp.is_none());
        assert!(data.gpu.is_none());
    }
}
```

- [ ] **Step 2: Run test**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml lhm
```

Expected:
```
test lhm::tests::lhm_start_with_missing_script_does_not_panic ... ok
test result: ok. 1 passed
```

- [ ] **Step 3: Declare module in main.rs**

```rust
mod lhm;
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/lhm.rs src-tauri/src/main.rs
git commit -m "feat: lhm.rs — persistent PS subprocess for temps/GPU, graceful degradation"
```

---

## Task 7: main.rs — Timer + Event Emission

**Files:**
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Replace main.rs with the full implementation**

```rust
// Prevents a console window from appearing on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::time;

mod lhm;
mod models;
mod monitor;

use lhm::LhmProcess;
use models::MetricsPayload;
use monitor::Monitor;

/// Tauri command: restart the app cleanly.
#[tauri::command]
fn restart_app(app: AppHandle) {
    app.restart();
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![restart_app])
        .setup(|app| {
            // Resolve path to read_temp.ps1 in the bundled resources.
            // In dev mode: resolves to hardware/ next to the exe.
            // In production: resolves inside the bundle's resource_dir.
            let resource_dir = app.path().resource_dir()?;
            let ps_script = resource_dir.join("hardware").join("read_temp.ps1");

            let lhm = Arc::new(Mutex::new(LhmProcess::start(&ps_script)));
            let monitor = Arc::new(Mutex::new(Monitor::new()));
            let app_handle = app.handle().clone();

            // Emit one tick immediately so the UI shows data at startup.
            {
                let mut mon = monitor.lock().unwrap();
                let lhm_data = lhm.lock().unwrap().get();
                let payload = build_payload(&mut mon, lhm_data);
                let _ = app_handle.emit("metrics-updated", payload);
            }

            // 2-second polling loop — runs for the lifetime of the app.
            tauri::async_runtime::spawn(async move {
                let mut interval = time::interval(Duration::from_secs(2));
                loop {
                    interval.tick().await;
                    let payload = {
                        let mut mon = monitor.lock().unwrap();
                        let lhm_data = lhm.lock().unwrap().get();
                        build_payload(&mut mon, lhm_data)
                    };
                    let _ = app_handle.emit("metrics-updated", payload);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_payload(mon: &mut Monitor, lhm_data: models::LhmData) -> MetricsPayload {
    MetricsPayload {
        cpu: models::CpuMetrics {
            percent: mon.cpu_percent(),
            temp: lhm_data.cpu_temp,
        },
        gpu: lhm_data.gpu,
        ram: mon.ram(),
        disks: mon.disks(),
        network: mon.network(),
    }
}
```

- [ ] **Step 2: Verify it compiles**

```powershell
cargo build --manifest-path src-tauri/Cargo.toml
```

Expected: `Finished dev` — no errors.

- [ ] **Step 3: Smoke test — verify events are emitted**

```powershell
cargo tauri dev
```

Open DevTools in the Tauri window (right-click → Inspect if dev build allows it, or check the Tauri debug console). You should see no JS errors related to metrics. Close the window.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "feat: main.rs — 2s Tokio timer emits metrics-updated to frontend"
```

---

## Task 8: Global CSS + Glassmorphism Tokens

**Files:**
- Modify: `src/app.css`

- [ ] **Step 1: Replace app.css**

```css
/* Reset */
*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

html, body {
  background: transparent;
  overflow: hidden;
  width: 320px;
  font-family: 'Consolas', 'Courier New', monospace;
  font-size: 13px;
  color: #c8c8c8;
  -webkit-font-smoothing: antialiased;
  user-select: none;
}

:root {
  --color-green:   #06d6a0;
  --color-yellow:  #ffd166;
  --color-red:     #ff6b6b;
  --color-cyan:    #74d7f7;
  --color-neutral: #c8c8c8;
  --color-dim:     #888;

  --bg-widget:     rgba(10, 10, 10, 0.78);
  --border-widget: rgba(255, 255, 255, 0.08);
  --radius:        12px;
  --padding-x:     16px;
  --padding-y:     14px;
  --row-gap:       5px;

  --bar-height:    4px;
  --bar-radius:    2px;
  --bar-bg:        rgba(255, 255, 255, 0.08);
}
```

- [ ] **Step 2: Commit**

```bash
git add src/app.css
git commit -m "feat: global CSS reset + glassmorphism design tokens"
```

---

## Task 9: colors.ts + formatRate — Pure Utilities

**Files:**
- Create: `src/lib/utils/colors.ts`

> These are pure functions. Write Vitest tests first.

- [ ] **Step 1: Install Vitest**

```powershell
npm install --save-dev vitest @vitest/ui
```

Add to `vite.config.ts` (inside the `defineConfig` object):
```typescript
test: {
  environment: 'node',
},
```

- [ ] **Step 2: Write the failing tests**

Create `src/lib/utils/colors.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { thresholdColor, netColor, formatRate } from './colors';

describe('thresholdColor', () => {
  it('returns green below 70', () => {
    expect(thresholdColor(0)).toBe('var(--color-green)');
    expect(thresholdColor(69.9)).toBe('var(--color-green)');
  });
  it('returns yellow at 70', () => {
    expect(thresholdColor(70)).toBe('var(--color-yellow)');
    expect(thresholdColor(89.9)).toBe('var(--color-yellow)');
  });
  it('returns red at 90', () => {
    expect(thresholdColor(90)).toBe('var(--color-red)');
    expect(thresholdColor(100)).toBe('var(--color-red)');
  });
});

describe('netColor', () => {
  it('upload below 1 MB/s → green', () => {
    expect(netColor(0.5, 'upload')).toBe('var(--color-green)');
  });
  it('download below 1 MB/s → cyan', () => {
    expect(netColor(0.5, 'download')).toBe('var(--color-cyan)');
  });
  it('upload 1-9 MB/s → yellow', () => {
    expect(netColor(5, 'upload')).toBe('var(--color-yellow)');
  });
  it('upload ≥ 10 MB/s → red', () => {
    expect(netColor(15, 'upload')).toBe('var(--color-red)');
  });
});

describe('formatRate', () => {
  it('formats bytes/s', () => {
    // 1/2048 MB/s = 0.5 KB/s → B/s path → Math.round(0.5 * 1024) = 512
    expect(formatRate(1 / 2048)).toBe('512 B/s');
  });
  it('formats KB/s', () => {
    // 0.5 MB/s → kbps = 512 → "512.0 KB/s"
    expect(formatRate(0.5)).toBe('512.0 KB/s');
  });
  it('formats MB/s', () => {
    expect(formatRate(2.5)).toBe('2.50 MB/s');
  });
});
```

- [ ] **Step 3: Run tests to verify they fail**

```powershell
npx vitest run src/lib/utils/colors.test.ts
```

Expected: FAIL — "Cannot find module './colors'"

- [ ] **Step 4: Implement colors.ts**

Create `src/lib/utils/colors.ts`:

```typescript
export type RateDirection = 'upload' | 'download';

/** Returns a CSS var color based on usage threshold (< 70 / 70-89 / ≥ 90). */
export function thresholdColor(percent: number): string {
  if (percent >= 90) return 'var(--color-red)';
  if (percent >= 70) return 'var(--color-yellow)';
  return 'var(--color-green)';
}

/** Returns a CSS var color for network rate (MB/s). Upload and download differ below 1 MB/s. */
export function netColor(mbps: number, dir: RateDirection): string {
  if (mbps >= 10) return 'var(--color-red)';
  if (mbps >= 1)  return 'var(--color-yellow)';
  return dir === 'download' ? 'var(--color-cyan)' : 'var(--color-green)';
}

/** Formats a rate in MB/s to a human-readable string. */
export function formatRate(mbps: number): string {
  if (mbps >= 1) return `${mbps.toFixed(2)} MB/s`;
  const kbps = mbps * 1024;
  if (kbps >= 1) return `${kbps.toFixed(1)} KB/s`;
  return `${Math.round(kbps * 1024)} B/s`;
}
```

- [ ] **Step 5: Run tests to verify they pass**

```powershell
npx vitest run src/lib/utils/colors.test.ts
```

Expected:
```
✓ colors.test.ts (8 tests)
Test Files  1 passed (1)
Tests  8 passed (8)
```

- [ ] **Step 6: Commit**

```bash
git add src/lib/utils/colors.ts src/lib/utils/colors.test.ts vite.config.ts package.json package-lock.json
git commit -m "feat: colors.ts utility functions with 8 Vitest tests passing"
```

---

## Task 10: metrics.svelte.ts — Reactive State Store

**Files:**
- Create: `src/lib/stores/metrics.svelte.ts`

> Svelte 5 runes (`$state`) replace writable stores. This file wires the Tauri event to the reactive state object.

- [ ] **Step 1: Create metrics.svelte.ts**

```typescript
import { listen } from '@tauri-apps/api/event';

export interface CpuMetrics {
  percent: number;
  temp: number | null;
}

export interface GpuMetrics {
  percent: number;
  temp: number | null;
  vram_used_gb: number;
  vram_total_gb: number;
}

export interface RamMetrics {
  percent: number;
  used_gb: number;
  total_gb: number;
}

export interface DiskInfo {
  mount: string;
  percent: number;
  used_gb: number;
  total_gb: number;
}

export interface NetworkMetrics {
  upload_mbps: number;
  download_mbps: number;
}

export interface MetricsPayload {
  cpu: CpuMetrics;
  gpu: GpuMetrics | null;
  ram: RamMetrics;
  disks: DiskInfo[];
  network: NetworkMetrics;
}

// Reactive global state — Svelte 5 runes.
// Components import this and use {metrics.cpu.percent} directly.
export const metrics = $state<MetricsPayload>({
  cpu: { percent: 0, temp: null },
  gpu: null,
  ram: { percent: 0, used_gb: 0, total_gb: 0 },
  disks: [],
  network: { upload_mbps: 0, download_mbps: 0 },
});

// Wire Tauri event to state — called once from App.svelte's onMount.
export async function startListening(): Promise<() => void> {
  const unlisten = await listen<MetricsPayload>('metrics-updated', (event) => {
    const p = event.payload;
    metrics.cpu = p.cpu;
    metrics.gpu = p.gpu;
    metrics.ram = p.ram;
    metrics.disks = p.disks;
    metrics.network = p.network;
  });
  return unlisten;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/stores/metrics.svelte.ts
git commit -m "feat: metrics.svelte.ts — $state store + Tauri event listener"
```

---

## Task 11: MetricRow.svelte — CPU / GPU / RAM Row

**Files:**
- Create: `src/lib/components/MetricRow.svelte`

- [ ] **Step 1: Create MetricRow.svelte**

```svelte
<script lang="ts">
  import { thresholdColor } from '$lib/utils/colors';

  interface Props {
    label: string;          // "CPU", "GPU", "RAM"
    percent: number;        // 0-100
    line2?: string;         // optional second value line (e.g. "9.4/16.0 G")
    temp?: number | null;   // temperature in °C, optional
  }

  let { label, percent, line2, temp }: Props = $props();

  const barColor = $derived(thresholdColor(percent));
</script>

<div class="row">
  <div class="label">{label}</div>
  <div class="bar-wrap">
    <div class="bar" style="width: {percent}%; background: {barColor};"></div>
  </div>
  <div class="values">
    <span class="pct" style="color: {barColor}">{percent.toFixed(0)}%</span>
    {#if temp != null}
      <span class="temp">{temp.toFixed(0)}°C</span>
    {/if}
    {#if line2}
      <span class="extra">{line2}</span>
    {/if}
  </div>
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 3.5em 1fr auto;
    align-items: center;
    gap: 8px;
    height: 20px;
  }
  .label {
    color: var(--color-dim);
    font-size: 11px;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .bar-wrap {
    height: var(--bar-height);
    background: var(--bar-bg);
    border-radius: var(--bar-radius);
    overflow: hidden;
  }
  .bar {
    height: 100%;
    border-radius: var(--bar-radius);
    transition: width 0.4s ease, background 0.3s ease;
  }
  .values {
    display: flex;
    gap: 6px;
    font-size: 11px;
    white-space: nowrap;
  }
  .pct  { font-weight: 600; min-width: 2.8em; text-align: right; }
  .temp { color: var(--color-dim); }
  .extra { color: var(--color-dim); }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/MetricRow.svelte
git commit -m "feat: MetricRow.svelte — label + smooth bar + values"
```

---

## Task 12: DiskRow.svelte

**Files:**
- Create: `src/lib/components/DiskRow.svelte`

- [ ] **Step 1: Create DiskRow.svelte**

```svelte
<script lang="ts">
  import { thresholdColor } from '$lib/utils/colors';
  import type { DiskInfo } from '$lib/stores/metrics.svelte';

  let { disk }: { disk: DiskInfo } = $props();

  const barColor = $derived(thresholdColor(disk.percent));
  const label = $derived(disk.mount.replace(/\\/g, '').replace(':', ':'));
</script>

<div class="row">
  <div class="label">{label}</div>
  <div class="bar-wrap">
    <div class="bar" style="width: {disk.percent}%; background: {barColor};"></div>
  </div>
  <div class="values">
    <span class="pct" style="color: {barColor}">{disk.percent.toFixed(0)}%</span>
    <span class="size">{disk.used_gb.toFixed(0)}/{disk.total_gb.toFixed(0)} G</span>
  </div>
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 3.5em 1fr auto;
    align-items: center;
    gap: 8px;
    height: 20px;
  }
  .label {
    color: var(--color-dim);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .bar-wrap {
    height: var(--bar-height);
    background: var(--bar-bg);
    border-radius: var(--bar-radius);
    overflow: hidden;
  }
  .bar {
    height: 100%;
    border-radius: var(--bar-radius);
    transition: width 0.4s ease, background 0.3s ease;
  }
  .values {
    display: flex;
    gap: 6px;
    font-size: 11px;
    white-space: nowrap;
  }
  .pct  { font-weight: 600; min-width: 2.8em; text-align: right; }
  .size { color: var(--color-dim); }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/DiskRow.svelte
git commit -m "feat: DiskRow.svelte — disk partition row"
```

---

## Task 13: NetworkRow.svelte

**Files:**
- Create: `src/lib/components/NetworkRow.svelte`

- [ ] **Step 1: Create NetworkRow.svelte**

```svelte
<script lang="ts">
  import { netColor, formatRate } from '$lib/utils/colors';
  import type { NetworkMetrics } from '$lib/stores/metrics.svelte';

  let { network }: { network: NetworkMetrics } = $props();

  const upColor   = $derived(netColor(network.upload_mbps, 'upload'));
  const downColor = $derived(netColor(network.download_mbps, 'download'));
  const upStr     = $derived(formatRate(network.upload_mbps));
  const downStr   = $derived(formatRate(network.download_mbps));
</script>

<div class="row">
  <span class="arrow" style="color: {upColor}">↑</span>
  <span class="rate" style="color: {upColor}">{upStr}</span>
  <span class="sep">·</span>
  <span class="arrow" style="color: {downColor}">↓</span>
  <span class="rate" style="color: {downColor}">{downStr}</span>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    height: 20px;
  }
  .arrow { font-weight: 700; }
  .rate  { min-width: 7em; }
  .sep   { color: var(--color-dim); margin: 0 4px; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/NetworkRow.svelte
git commit -m "feat: NetworkRow.svelte — upload/download with rate formatting"
```

---

## Task 14: App.svelte — Full Integration

**Files:**
- Modify: `src/App.svelte`
- Modify: `src/main.ts`

- [ ] **Step 1: Replace App.svelte**

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { metrics, startListening } from '$lib/stores/metrics.svelte';
  import MetricRow from '$lib/components/MetricRow.svelte';
  import DiskRow from '$lib/components/DiskRow.svelte';
  import NetworkRow from '$lib/components/NetworkRow.svelte';

  let unlisten: (() => void) | null = null;

  onMount(async () => {
    // Always-on-bottom: keep the widget below all other windows.
    const win = getCurrentWindow();
    await win.setAlwaysOnBottom(true);

    // Start receiving metrics from Rust backend.
    unlisten = await startListening();
  });

  onDestroy(() => {
    unlisten?.();
  });

  const ramLine2 = $derived(
    `${metrics.ram.used_gb.toFixed(1)}/${metrics.ram.total_gb.toFixed(1)} G`
  );
  const gpuLine2 = $derived(
    metrics.gpu
      ? `${metrics.gpu.vram_used_gb.toFixed(1)}/${metrics.gpu.vram_total_gb.toFixed(1)} G`
      : undefined
  );
</script>

<!-- data-tauri-drag-region: clicking and dragging this div moves the window -->
<div class="widget" data-tauri-drag-region>
  <!-- CPU -->
  <MetricRow label="CPU" percent={metrics.cpu.percent} temp={metrics.cpu.temp} />

  <!-- GPU (only shown if hardware detected) -->
  {#if metrics.gpu !== null}
    <MetricRow
      label="GPU"
      percent={metrics.gpu.percent}
      temp={metrics.gpu.temp}
      line2={gpuLine2}
    />
  {/if}

  <!-- RAM -->
  <MetricRow label="RAM" percent={metrics.ram.percent} line2={ramLine2} />

  <!-- Separator -->
  <div class="sep"></div>

  <!-- Disks -->
  {#each metrics.disks as disk (disk.mount)}
    <DiskRow {disk} />
  {/each}

  <!-- Network -->
  <NetworkRow network={metrics.network} />
</div>

<style>
  .widget {
    position: fixed;
    top: 0;
    left: 0;
    display: flex;
    flex-direction: column;
    gap: var(--row-gap);
    background: var(--bg-widget);
    backdrop-filter: blur(16px);
    -webkit-backdrop-filter: blur(16px);
    border: 1px solid var(--border-widget);
    border-radius: var(--radius);
    padding: var(--padding-y) var(--padding-x);
    width: fit-content;
    min-width: 220px;
    cursor: default;
  }
  .sep {
    height: 1px;
    background: rgba(255, 255, 255, 0.06);
    margin: 2px 0;
  }
</style>
```

- [ ] **Step 2: Update main.ts to import app.css**

Ensure `src/main.ts` contains:
```typescript
import './app.css';
import App from './App.svelte';
import { mount } from 'svelte';

const app = mount(App, {
  target: document.getElementById('app')!,
});

export default app;
```

> Note: Svelte 5 uses `mount()` instead of `new App(...)`. If the scaffolded main.ts already uses `mount`, keep it as-is.

- [ ] **Step 3: Verify the widget renders**

```powershell
cargo tauri dev
```

Expected: The glassmorphism widget appears in the top-left corner of the screen. CPU, RAM, disks, and network rows are visible with colored bars. GPU row only appears if the machine has a supported GPU.

- [ ] **Step 4: Check for TypeScript errors**

```powershell
npx tsc --noEmit
```

Expected: No errors.

- [ ] **Step 5: Run all tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
npx vitest run
```

Expected: All 9 Rust tests pass, 8 Vitest tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/App.svelte src/main.ts
git commit -m "feat: App.svelte — full widget integration, all metrics displayed"
```

---

## Task 15: Position Persistence via tauri-plugin-store

**Files:**
- Modify: `src/App.svelte`

> The widget position (x, y) is saved to `%APPDATA%\sysmon-widget\config.json` when the user releases the drag. It's loaded on startup to restore the position.

- [ ] **Step 1: Install the JS plugin**

```powershell
npm install @tauri-apps/plugin-store
```

- [ ] **Step 2: Add position persistence to App.svelte**

Add to the `<script>` block in `App.svelte`:

```typescript
import { load } from '@tauri-apps/plugin-store';

// On mount: load saved position
onMount(async () => {
  const win = getCurrentWindow();
  await win.setAlwaysOnBottom(true);
  unlisten = await startListening();

  // Restore saved position
  try {
    const store = await load('config.json', { autoSave: false });
    const x = await store.get<number>('x');
    const y = await store.get<number>('y');
    if (x != null && y != null) {
      await win.setPosition({ type: 'Physical', x, y });
    }
  } catch {
    // No saved position — use tauri.conf.json defaults (50, 50)
  }
});
```

Add a `mouseup` handler to save position on drag end (add to `.widget` div):
```svelte
<div class="widget" data-tauri-drag-region onmouseup={savePosition}>
```

Add the `savePosition` function to the script:
```typescript
async function savePosition() {
  try {
    const win = getCurrentWindow();
    const pos = await win.outerPosition();
    const store = await load('config.json', { autoSave: false });
    await store.set('x', pos.x);
    await store.set('y', pos.y);
    await store.save();
  } catch {
    // Non-fatal — just skip saving if it fails
  }
}
```

- [ ] **Step 3: Add store capability permission**

In `src-tauri/capabilities/default.json`, add:
```json
"store:allow-load",
"store:allow-set",
"store:allow-save",
"store:allow-get"
```

- [ ] **Step 4: Test position persistence**

```powershell
cargo tauri dev
```

Drag the widget to a new position. Close it. Relaunch with `cargo tauri dev`. Expected: widget opens at the saved position.

- [ ] **Step 5: Commit**

```bash
git add src/App.svelte src-tauri/capabilities/default.json package.json package-lock.json
git commit -m "feat: position persistence via tauri-plugin-store"
```

---

## Task 16: Minimal Context Menu (Restart + Quit)

> Full context menu (startup toggle, auto-update) is in Plan 2. This task adds just Restart and Quit so the widget is usable in dev.

**Files:**
- Create: `src/lib/components/ContextMenu.svelte`
- Modify: `src/App.svelte`

- [ ] **Step 1: Create ContextMenu.svelte**

```svelte
<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    x: number;
    y: number;
    onclose: () => void;
  }

  let { x, y, onclose }: Props = $props();

  async function restart() {
    onclose();
    await invoke('restart_app');
  }

  async function quit() {
    onclose();
    const win = getCurrentWindow();
    await win.close();
  }
</script>

<!-- Backdrop: clicking outside closes the menu -->
<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div class="backdrop" onclick={onclose}></div>

<menu class="menu" style="left: {x}px; top: {y}px;">
  <li class="item" onclick={restart}>Redémarrer</li>
  <li class="sep"></li>
  <li class="item" onclick={quit}>Quitter</li>
</menu>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 9;
  }
  .menu {
    position: fixed;
    z-index: 10;
    list-style: none;
    background: rgba(18, 18, 18, 0.92);
    backdrop-filter: blur(12px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 8px;
    padding: 4px 0;
    min-width: 160px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
    font-size: 12px;
    color: #c8c8c8;
  }
  .item {
    padding: 7px 14px;
    cursor: pointer;
    transition: background 0.15s;
  }
  .item:hover {
    background: rgba(255, 255, 255, 0.08);
    color: #fff;
  }
  .sep {
    height: 1px;
    background: rgba(255, 255, 255, 0.06);
    margin: 4px 0;
  }
</style>
```

- [ ] **Step 2: Wire context menu to App.svelte**

Add to the `<script>` block:
```typescript
import ContextMenu from '$lib/components/ContextMenu.svelte';

let menuOpen = $state(false);
let menuX = $state(0);
let menuY = $state(0);

function onContextMenu(e: MouseEvent) {
  e.preventDefault();
  menuX = e.clientX;
  menuY = e.clientY;
  menuOpen = true;
}
```

Add to the template (after `.widget` closing tag):
```svelte
<div class="widget" data-tauri-drag-region onmouseup={savePosition} oncontextmenu={onContextMenu}>
  ...
</div>

{#if menuOpen}
  <ContextMenu x={menuX} y={menuY} onclose={() => menuOpen = false} />
{/if}
```

- [ ] **Step 3: Test context menu**

```powershell
cargo tauri dev
```

Right-click the widget → menu appears. Click "Redémarrer" → widget closes and reopens. Click "Quitter" → widget closes.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/ContextMenu.svelte src/App.svelte
git commit -m "feat: minimal context menu — Restart + Quit (Plan 2 adds startup toggle, updater)"
```

---

## Final Verification

- [ ] **Run all Rust tests**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: 9 tests pass.

- [ ] **Run all Vitest tests**

```powershell
npx vitest run
```

Expected: 8 tests pass.

- [ ] **TypeScript check**

```powershell
npx tsc --noEmit
```

Expected: no errors.

- [ ] **Full dev smoke test**

```powershell
cargo tauri dev
```

Checklist:
- [ ] Widget appears in top-left, glassmorphism background visible
- [ ] CPU% + temperature (if LHM works) updates every 2s
- [ ] GPU row visible if machine has supported GPU
- [ ] RAM with used/total GB values
- [ ] Disk partitions listed (C:\\ at minimum)
- [ ] Network ↑/↓ with formatted rates
- [ ] Drag to new position, close, reopen → position restored
- [ ] Right-click → menu → Restart → widget reopens
- [ ] Right-click → menu → Quit → widget closes

- [ ] **Build release exe**

```powershell
cargo tauri build
```

Expected: `dist/SysmonWidget_2.0.0_x64-setup.exe` (NSIS installer) and/or `dist/SysmonWidget_2.0.0_x64_en-US.msi` created. Roughly 8–15 MB.

- [ ] **Final commit**

```bash
git add -A
git commit -m "chore: Plan 1 complete — core widget v2.0.0 (Tauri + Svelte 5)"
git push origin tauri-rewrite
```

---

## What's Next — Plan 2

Plan 2 covers system integrations and distribution:
- `startup.rs` + Tauri commands → "Démarrer avec Windows" toggle in context menu
- `tauri-plugin-updater` → auto-update from GitHub Releases (replaces updater.py + PS scripts)
- UAC `app.manifest` with `requireAdministrator`
- GitHub Actions `release.yml` using `tauri-apps/tauri-action`
- `tauri signer generate` → signing key + GitHub Secret setup
