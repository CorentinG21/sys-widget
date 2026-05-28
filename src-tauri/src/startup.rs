use std::process::Command;

const TASK_NAME: &str = "SysmonWidget";

/// Returns true if the SysmonWidget scheduled task exists.
pub fn is_registered() -> bool {
    Command::new("schtasks")
        .args(["/Query", "/TN", TASK_NAME])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Creates an ONLOGON task that runs the given exe with highest privileges.
/// Returns true on success.
pub fn register(exe_path: &str) -> bool {
    // Build a one-liner PowerShell command to create the task with RL HIGHEST.
    // schtasks /Create doesn't support RunLevel directly — use New-ScheduledTask.
    let ps = format!(
        r#"$action  = New-ScheduledTaskAction  -Execute '{exe}';
$trigger = New-ScheduledTaskTrigger  -AtLogOn;
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries;
$principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -RunLevel Highest;
Register-ScheduledTask -TaskName '{name}' -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Force | Out-Null"#,
        exe = exe_path,
        name = TASK_NAME,
    );

    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &ps])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Removes the scheduled task. Returns true on success.
pub fn unregister() -> bool {
    Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &format!(
                "Unregister-ScheduledTask -TaskName '{}' -Confirm:$false",
                TASK_NAME
            ),
        ])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Toggle: register if not registered, unregister otherwise.
/// Returns the new state (true = registered).
pub fn toggle(exe_path: &str) -> bool {
    if is_registered() {
        unregister();
        false
    } else {
        register(exe_path);
        true
    }
}
