use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Prevents a console window from appearing when spawning subprocesses.
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

use serde::Deserialize;

use crate::models::{GpuMetrics, LhmData};

// ─── Raw structs matching read_temp.ps1 JSON output ──────────────────────────
//
// The PS script emits:
//   { "cpu_temp": 45.0,
//     "gpu": { "usage": 30.0, "temp": 65.0,
//              "vram_used_mb": 2048.0, "vram_total_mb": 8192.0 } }
//
// Field names and units differ from GpuMetrics (which uses bytes + "percent").

#[derive(Debug, Deserialize)]
struct RawGpu {
    usage: Option<f32>,
    temp: Option<f32>,
    vram_used_mb: Option<f32>,
    vram_total_mb: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct RawLhm {
    cpu_temp: Option<f32>,
    gpu: Option<RawGpu>,
}

impl From<RawLhm> for LhmData {
    fn from(raw: RawLhm) -> Self {
        let gpu = raw.gpu.and_then(|g| {
            // Only expose GPU if we have at least a load reading.
            let percent = g.usage?;
            Some(GpuMetrics {
                percent,
                temp: g.temp,
                // PS reports VRAM in MB; convert to bytes.
                vram_used: g.vram_used_mb.unwrap_or(0.0) as u64 * 1_048_576,
                vram_total: g.vram_total_mb.unwrap_or(0.0) as u64 * 1_048_576,
            })
        });
        LhmData {
            cpu_temp: raw.cpu_temp,
            gpu,
        }
    }
}

// ─── LhmProcess ──────────────────────────────────────────────────────────────

/// Manages a persistent PowerShell subprocess running `read_temp.ps1`.
pub struct LhmProcess {
    child: Child,
    pub data: Arc<Mutex<LhmData>>,
}

impl LhmProcess {
    /// Spawn the PowerShell subprocess.
    pub fn start(script_path: PathBuf) -> Result<Self, String> {
        let mut cmd = Command::new("powershell");
        cmd.args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                script_path.to_str().ok_or("invalid script path")?,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        // Hide the PowerShell console window on Windows.
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn LHM subprocess: {e}"))?;

        let stdout = child.stdout.take().ok_or("could not capture stdout")?;

        let data: Arc<Mutex<LhmData>> = Arc::new(Mutex::new(LhmData::default()));
        let data_clone = Arc::clone(&data);

        // Background reader thread: parse JSON lines and update shared state.
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                let line = line.trim().to_owned();
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_str::<RawLhm>(&line) {
                    Ok(raw) => {
                        if let Ok(mut guard) = data_clone.lock() {
                            *guard = raw.into();
                        }
                    }
                    Err(e) => {
                        eprintln!("[lhm] JSON parse error: {e} — line: {line}");
                    }
                }
            }
        });

        Ok(Self { child, data })
    }

    /// Return a clone of the latest LHM snapshot.
    pub fn latest(&self) -> LhmData {
        self.data.lock().map(|g| g.clone()).unwrap_or_default()
    }

    /// Kill the subprocess. Call this before process exit.
    pub fn cleanup(&mut self) {
        let _ = self.child.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_lhm_converts_to_lhm_data() {
        let json = r#"{"cpu_temp":55.0,"gpu":{"usage":30.0,"temp":65.0,"vram_used_mb":2048.0,"vram_total_mb":8192.0}}"#;
        let raw: RawLhm = serde_json::from_str(json).unwrap();
        let data: LhmData = raw.into();
        assert_eq!(data.cpu_temp, Some(55.0));
        let gpu = data.gpu.unwrap();
        assert_eq!(gpu.percent, 30.0);
        assert_eq!(gpu.temp, Some(65.0));
        assert_eq!(gpu.vram_used,  2048_u64 * 1_048_576);
        assert_eq!(gpu.vram_total, 8192_u64 * 1_048_576);
    }

    #[test]
    fn null_gpu_usage_hides_gpu() {
        let json = r#"{"cpu_temp":45.0,"gpu":{"usage":null,"temp":70.0,"vram_used_mb":null,"vram_total_mb":null}}"#;
        let raw: RawLhm = serde_json::from_str(json).unwrap();
        let data: LhmData = raw.into();
        assert!(data.gpu.is_none(), "GPU with null usage should be hidden");
    }

    #[test]
    fn null_gpu_field_gives_none() {
        let json = r#"{"cpu_temp":40.0,"gpu":null}"#;
        let raw: RawLhm = serde_json::from_str(json).unwrap();
        let data: LhmData = raw.into();
        assert!(data.gpu.is_none());
    }
}
