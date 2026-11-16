//! Removal of the per-user install left behind by the move to per-machine.
//!
//! Up to 2.9.0 the app was installed per user (AppData\Local); since 2.10.0 it
//! is installed per machine (Program Files). The NSIS updater installs the new
//! version without touching the old one, whose shortcuts keep launching it.
//! `hardware/cleanup_legacy.ps1` does the actual work; see its header.

use std::path::Path;
use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Prevents a console window from appearing when spawning subprocesses.
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Runs the cleanup script and logs what it did. Blocking: call it off the
/// main thread. Release builds only: the script itself also checks that it is
/// started from the per-machine install, so shortcuts never point at a dev exe.
pub fn remove_per_user_install(script: &Path, current_exe: &str) {
    let Some(script) = script.to_str() else {
        return;
    };
    if !Path::new(script).exists() {
        eprintln!("[legacy] {script} not found, skipped");
        return;
    }

    let mut cmd = Command::new("powershell");
    cmd.args(cleanup_args(script, current_exe));

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    match cmd.output() {
        Ok(output) => {
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                eprintln!("[legacy] {line}");
            }
            for line in String::from_utf8_lossy(&output.stderr).lines() {
                eprintln!("[legacy:stderr] {line}");
            }
        }
        Err(e) => eprintln!("[legacy] could not run the cleanup script: {e}"),
    }
}

/// Command-line arguments for `powershell.exe` to run the cleanup script.
fn cleanup_args(script: &str, current_exe: &str) -> Vec<String> {
    [
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        script,
        "-CurrentExe",
        current_exe,
    ]
    .map(String::from)
    .to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_receives_the_running_exe_as_a_single_argument() {
        let exe = r"C:\Program Files\SysmonWidget\sysmon-widget.exe";
        let args = cleanup_args(r"C:\Program Files\SysmonWidget\hardware\cleanup_legacy.ps1", exe);
        let file = args
            .iter()
            .position(|a| a == "-File")
            .expect("-File argument");
        assert_eq!(
            args[file + 1],
            r"C:\Program Files\SysmonWidget\hardware\cleanup_legacy.ps1"
        );
        // After the script path, so PowerShell passes it to the script; one
        // element, so the space in "Program Files" does not split it.
        assert_eq!(args[file + 2..], ["-CurrentExe", exe]);
    }
}
