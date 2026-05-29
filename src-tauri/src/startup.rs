use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Prevents a console window from appearing when spawning subprocesses.
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const TASK_NAME: &str = "SysmonWidget";

/// Returns true if the SysmonWidget scheduled task exists.
pub fn is_registered() -> bool {
    let mut cmd = Command::new("schtasks");
    cmd.args(["/Query", "/TN", TASK_NAME]);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Creates an ONLOGON task that runs the given exe with highest privileges.
/// Returns true on success.
pub(crate) fn register(exe_path: &str) -> bool {
    let ps = format!(
        r#"$action   = New-ScheduledTaskAction  -Execute '{exe}';
$trigger  = New-ScheduledTaskTrigger  -AtLogOn;
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries;
$principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -RunLevel Highest;
Register-ScheduledTask -TaskName '{name}' -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Force | Out-Null"#,
        exe = exe_path,
        name = TASK_NAME,
    );

    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &ps]);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.output()
        .map(|o| {
            if !o.status.success() {
                eprintln!("[startup] register failed: {}", String::from_utf8_lossy(&o.stderr));
            }
            o.status.success()
        })
        .unwrap_or(false)
}

/// Removes the scheduled task. Returns true on success.
pub(crate) fn unregister() -> bool {
    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        &format!(
            "Unregister-ScheduledTask -TaskName '{}' -Confirm:$false",
            TASK_NAME
        ),
    ]);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Toggle: register if not registered, unregister otherwise.
/// Returns the actual new state (true = registered) — reflects whether the
/// schtask command succeeded, not just the intended state.
pub fn toggle(exe_path: &str) -> bool {
    if is_registered() {
        // unregister() returns true on success → task removed → state = false
        !unregister()
    } else {
        // register() returns true on success → task added → state = true
        register(exe_path)
    }
}
