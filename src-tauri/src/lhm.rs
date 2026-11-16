use std::{
    io::{BufRead, BufReader, Read},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::Instant,
};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Prevents a console window from appearing when spawning subprocesses.
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

use serde::Deserialize;

use crate::models::{GpuMetrics, LhmData};

/// After this many seconds without a valid JSON line, `latest()` returns None temps.
const STALE_SECS: u64 = 5;

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
    data: Arc<Mutex<LhmData>>,
    /// Wall-clock time of the last successfully parsed JSON line.
    last_seen: Arc<Mutex<Option<Instant>>>,
}

impl LhmProcess {
    /// Spawn the PowerShell subprocess.
    pub fn start(script_path: PathBuf) -> Result<Self, String> {
        let script = script_path.to_str().ok_or("invalid script path")?;
        let mut cmd = Command::new("powershell");
        cmd.args(powershell_args(script, std::process::id()))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Hide the PowerShell console window on Windows.
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn LHM subprocess: {e}"))?;

        let stdout = child.stdout.take().ok_or("could not capture stdout")?;

        // The script's errors used to be discarded, which hid for months that
        // the sensor library was not loading at all.
        if let Some(stderr) = child.stderr.take() {
            thread::spawn(move || log_stderr(stderr));
        }

        let data: Arc<Mutex<LhmData>> = Arc::new(Mutex::new(LhmData::default()));
        let data_clone = Arc::clone(&data);

        let last_seen: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
        let last_seen_clone = Arc::clone(&last_seen);

        // Background reader thread: parse JSON lines and update shared state.
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                match serde_json::from_str::<RawLhm>(line) {
                    Ok(raw) => {
                        if let Ok(mut guard) = data_clone.lock() {
                            *guard = raw.into();
                        }
                        if let Ok(mut ts) = last_seen_clone.lock() {
                            *ts = Some(Instant::now());
                        }
                    }
                    Err(e) => {
                        eprintln!("[lhm] JSON parse error: {e} — line: {line}");
                    }
                }
            }
            eprintln!("[lhm] reader thread exited (subprocess may have stopped)");
        });

        Ok(Self { child, data, last_seen })
    }

    /// Return a clone of the latest LHM snapshot.
    /// Returns `LhmData::default()` (None temps) if no valid data received in `STALE_SECS`.
    pub fn latest(&self) -> LhmData {
        let is_stale = self
            .last_seen
            .lock()
            .map(|ts| ts.map_or(true, |t| t.elapsed().as_secs() >= STALE_SECS))
            .unwrap_or(true);
        if is_stale {
            return LhmData::default();
        }
        self.data.lock().map(|g| g.clone()).unwrap_or_default()
    }

    /// Returns true if the subprocess is still running.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Kill the current subprocess and spawn a fresh one from the same script.
    pub fn restart(&mut self, script_path: &PathBuf) -> Result<(), String> {
        let _ = self.child.kill();
        let new = LhmProcess::start(script_path.clone())?;
        self.child = new.child;
        self.data = new.data;
        self.last_seen = new.last_seen;
        eprintln!("[lhm] subprocess restarted");
        Ok(())
    }

    /// Kill the subprocess. Call this before process exit.
    pub fn cleanup(&mut self) {
        let _ = self.child.kill();
    }
}

/// Command-line arguments for `powershell.exe` to run `script`.
///
/// `parent_pid` is handed to the script so it can stop by itself once that
/// process is gone. Killing the child from here only covers clean exits; a
/// crash, a forced kill or the updater replacing the app would otherwise leave
/// PowerShell running forever (writing to a closed pipe does not stop it).
fn powershell_args(script: &str, parent_pid: u32) -> Vec<String> {
    [
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        script,
        "-ParentPid",
        &parent_pid.to_string(),
    ]
    .map(String::from)
    .to_vec()
}

/// Forwards the script's error output to our log until the pipe closes.
/// Reads bytes, not UTF-8 lines: PowerShell writes in the console code page.
fn log_stderr(stderr: impl Read) {
    let mut reader = BufReader::new(stderr);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let line = String::from_utf8_lossy(&buf);
                let line = line.trim_end();
                if !line.is_empty() {
                    eprintln!("[lhm:stderr] {line}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_is_told_which_process_to_watch() {
        let args = powershell_args(r"C:\app\hardware\read_temp.ps1", 4242);
        let file = args
            .iter()
            .position(|a| a == "-File")
            .expect("-File argument");
        assert_eq!(args[file + 1], r"C:\app\hardware\read_temp.ps1");
        // Script parameters must follow the script path: placed before -File,
        // PowerShell would take them for its own options.
        assert_eq!(args[file + 2..], ["-ParentPid", "4242"]);
    }

    #[test]
    fn raw_lhm_converts_to_lhm_data() {
        let json = r#"{"cpu_temp":55.0,"gpu":{"usage":30.0,"temp":65.0,"vram_used_mb":2048.0,"vram_total_mb":8192.0}}"#;
        let raw: RawLhm = serde_json::from_str(json).unwrap();
        let data: LhmData = raw.into();
        assert_eq!(data.cpu_temp, Some(55.0));
        let gpu = data.gpu.unwrap();
        assert_eq!(gpu.percent, 30.0);
        assert_eq!(gpu.temp, Some(65.0));
        assert_eq!(gpu.vram_used, 2048_u64 * 1_048_576);
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
