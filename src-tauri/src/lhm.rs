use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};

use crate::models::LhmData;

/// Manages a persistent PowerShell subprocess running `read_temp.ps1`.
/// The subprocess emits one JSON line every ~2 s:
///   {"cpu_temp": 45.0, "gpu": {"name": "...", "percent": 30.0, ...}}
pub struct LhmProcess {
    child: Child,
    pub data: Arc<Mutex<LhmData>>,
}

impl LhmProcess {
    /// Spawn the PowerShell subprocess.
    /// `script_path` must point to the bundled `read_temp.ps1`.
    pub fn start(script_path: PathBuf) -> Result<Self, String> {
        let mut child = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                script_path
                    .to_str()
                    .ok_or("invalid script path")?,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("failed to spawn LHM subprocess: {e}"))?;

        let stdout = child
            .stdout
            .take()
            .ok_or("could not capture stdout")?;

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
                match serde_json::from_str::<LhmData>(&line) {
                    Ok(parsed) => {
                        if let Ok(mut guard) = data_clone.lock() {
                            *guard = parsed;
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

    /// Smoke-test: default LhmData serialises / deserialises correctly.
    #[test]
    fn lhm_data_roundtrip() {
        let data = LhmData {
            cpu_temp: Some(55.0),
            gpu: None,
        };
        let json = serde_json::to_string(&data).unwrap();
        let back: LhmData = serde_json::from_str(&json).unwrap();
        assert_eq!(back.cpu_temp, Some(55.0));
        assert!(back.gpu.is_none());
    }
}
