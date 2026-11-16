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

    cmd.output().map(|o| o.status.success()).unwrap_or(false)
}

/// Creates an ONLOGON task that runs the given exe with highest privileges.
/// Returns true on success.
pub(crate) fn register(exe_path: &str) -> bool {
    let ps = register_script(exe_path);

    let mut cmd = Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        &ps,
    ]);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    cmd.output()
        .map(|o| {
            if !o.status.success() {
                eprintln!(
                    "[startup] register failed: {}",
                    String::from_utf8_lossy(&o.stderr)
                );
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

    cmd.output().map(|o| o.status.success()).unwrap_or(false)
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

/// Re-registers the startup task when it is out of date: created by an older
/// version (72-hour execution limit, see `register_script`) or pointing at a
/// previous install location. Does nothing when no task is registered.
///
/// Spawns `schtasks` (and PowerShell when a repair is needed): call it off the
/// main thread.
pub fn repair_if_outdated(exe_path: &str) {
    let Some(xml) = export_task_xml() else {
        return; // not registered: the user never enabled "start with Windows"
    };
    if task_is_current(&xml, exe_path) {
        return;
    }
    eprintln!("[startup] scheduled task is outdated, re-registering for {exe_path}");
    if !register(exe_path) {
        eprintln!("[startup] could not update the scheduled task");
    }
}

/// `schtasks /Query /XML` output for the task, or None when it does not exist.
fn export_task_xml() -> Option<String> {
    let mut cmd = Command::new("schtasks");
    cmd.args(["/Query", "/TN", TASK_NAME, "/XML"]);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    // schtasks writes in the console code page; the parts compared here are
    // ASCII unless the install path itself contains accented characters.
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Wraps `s` in a PowerShell single-quoted string literal.
fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// PowerShell script that creates (or replaces) the logon task for `exe_path`.
fn register_script(exe_path: &str) -> String {
    // -ExecutionTimeLimit Zero: without it Task Scheduler applies its default
    // of 72 hours and kills the widget after three days of uptime.
    format!(
        r#"$action   = New-ScheduledTaskAction  -Execute {exe};
$trigger  = New-ScheduledTaskTrigger  -AtLogOn;
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit ([TimeSpan]::Zero);
$principal = New-ScheduledTaskPrincipal -UserId $env:USERNAME -RunLevel Highest;
Register-ScheduledTask -TaskName {name} -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Force | Out-Null"#,
        exe = ps_quote(exe_path),
        name = ps_quote(TASK_NAME),
    )
}

/// Text of the first `<tag>…</tag>` element of `xml`, with XML entities decoded.
fn xml_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = start + xml[start..].find(&close)?;
    Some(
        xml[start..end]
            .trim()
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&"),
    )
}

/// True when the registered task, as exported by `schtasks /Query /XML`,
/// launches `exe_path` and has no execution time limit.
fn task_is_current(task_xml: &str, exe_path: &str) -> bool {
    let same_exe = xml_text(task_xml, "Command")
        .is_some_and(|command| command.trim_matches('"').eq_ignore_ascii_case(exe_path));
    // "PT0S" is how Task Scheduler writes "no limit"; a missing element means 72 hours.
    let unlimited = xml_text(task_xml, "ExecutionTimeLimit").is_some_and(|limit| limit == "PT0S");
    same_exe && unlimited
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Export of the task as versions <= 2.9.0 registered it (captured from a
    /// real machine): per-user install path, and no `ExecutionTimeLimit`
    /// element, which means the Task Scheduler default of 72 hours.
    const LEGACY_TASK: &str = r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.3" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <URI>\SysmonWidget</URI>
  </RegistrationInfo>
  <Principals>
    <Principal id="Author">
      <UserId>S-1-5-21-1111111111-2222222222-3333333333-1001</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <UseUnifiedSchedulingEngine>true</UseUnifiedSchedulingEngine>
  </Settings>
  <Triggers>
    <LogonTrigger />
  </Triggers>
  <Actions Context="Author">
    <Exec>
      <Command>C:\Users\coco\AppData\Local\SysmonWidget\sysmon-widget.exe</Command>
    </Exec>
  </Actions>
</Task>"#;

    fn task_xml(command: &str, limit: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.3" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Settings>
    <ExecutionTimeLimit>{limit}</ExecutionTimeLimit>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{command}</Command>
    </Exec>
  </Actions>
</Task>"#
        )
    }

    const LEGACY_EXE: &str = r"C:\Users\coco\AppData\Local\SysmonWidget\sysmon-widget.exe";
    const MACHINE_EXE: &str = r"C:\Program Files\SysmonWidget\sysmon-widget.exe";

    #[test]
    fn ps_quote_wraps_a_path_with_spaces() {
        assert_eq!(
            ps_quote(MACHINE_EXE),
            r"'C:\Program Files\SysmonWidget\sysmon-widget.exe'"
        );
    }

    #[test]
    fn ps_quote_doubles_apostrophes_so_the_path_cannot_end_the_string() {
        assert_eq!(
            ps_quote(r"C:\Users\O'Brien\sysmon-widget.exe"),
            r"'C:\Users\O''Brien\sysmon-widget.exe'"
        );
    }

    #[test]
    fn register_script_runs_the_exe_given_even_with_an_apostrophe_in_its_path() {
        let script = register_script(r"C:\Users\O'Brien\sysmon-widget.exe");
        assert!(
            script.contains(r"-Execute 'C:\Users\O''Brien\sysmon-widget.exe'"),
            "script={script}"
        );
    }

    #[test]
    fn register_script_disables_the_72_hour_execution_limit() {
        let script = register_script(MACHINE_EXE);
        assert!(
            script.contains("-ExecutionTimeLimit ([TimeSpan]::Zero)"),
            "script={script}"
        );
    }

    #[test]
    fn task_without_time_limit_pointing_at_this_exe_is_current() {
        assert!(task_is_current(&task_xml(MACHINE_EXE, "PT0S"), MACHINE_EXE));
    }

    #[test]
    fn path_comparison_ignores_case() {
        let xml = task_xml(r"c:\program files\sysmonwidget\SYSMON-WIDGET.EXE", "PT0S");
        assert!(task_is_current(&xml, MACHINE_EXE));
    }

    #[test]
    fn xml_entities_in_the_command_are_decoded() {
        let xml = task_xml(r"C:\Tools &amp; Apps\sysmon-widget.exe", "PT0S");
        assert!(task_is_current(&xml, r"C:\Tools & Apps\sysmon-widget.exe"));
    }

    #[test]
    fn legacy_task_is_outdated_after_moving_to_program_files() {
        assert!(!task_is_current(LEGACY_TASK, MACHINE_EXE));
    }

    #[test]
    fn legacy_task_is_outdated_even_for_its_own_exe_because_of_the_default_limit() {
        assert!(!task_is_current(LEGACY_TASK, LEGACY_EXE));
    }

    #[test]
    fn task_with_a_72_hour_limit_is_outdated() {
        assert!(!task_is_current(&task_xml(MACHINE_EXE, "PT72H"), MACHINE_EXE));
    }

    #[test]
    fn task_pointing_at_another_exe_is_outdated() {
        assert!(!task_is_current(&task_xml(LEGACY_EXE, "PT0S"), MACHINE_EXE));
    }

    #[test]
    fn unreadable_export_is_outdated() {
        assert!(!task_is_current("", MACHINE_EXE));
        assert!(!task_is_current("ERROR: The system cannot find the file specified.", MACHINE_EXE));
    }
}
