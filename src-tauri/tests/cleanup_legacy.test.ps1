# Tests for hardware/cleanup_legacy.ps1. Plain Windows PowerShell 5.1, no Pester.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File src-tauri\tests\cleanup_legacy.test.ps1
#
# Exit code = number of failed tests.
#
# Every test builds a fake machine in a temp folder and a throwaway registry
# key under HKCU:\Software\SysmonWidgetTests, then runs the real script on it.
# Nothing outside those two places is touched.

$scriptPath = Join-Path $PSScriptRoot '..\hardware\cleanup_legacy.ps1' | Resolve-Path | Select-Object -ExpandProperty Path
$script:failures = 0

function Test-Case {
    param([string]$Name, [scriptblock]$Body)
    $script:fx = New-Fixture
    try {
        & $Body
        Write-Host "PASS  $Name"
    } catch {
        $script:failures++
        Write-Host "FAIL  $Name"
        Write-Host "      $($_.Exception.Message)"
    } finally {
        Remove-Fixture $script:fx
    }
}

function Assert-True  { param($Value, [string]$Message) if (-not $Value) { throw $Message } }
function Assert-False { param($Value, [string]$Message) if ($Value)     { throw $Message } }
function Assert-Equal {
    param($Actual, $Expected, [string]$What)
    if ($Actual -ne $Expected) { throw "$What : expected '$Expected', got '$Actual'" }
}

$shell = New-Object -ComObject WScript.Shell

function New-Shortcut {
    param([string]$Path, [string]$Target)
    New-Item -ItemType Directory -Path (Split-Path $Path) -Force | Out-Null
    $lnk = $shell.CreateShortcut($Path)
    $lnk.TargetPath = $Target
    $lnk.WorkingDirectory = Split-Path $Target
    $lnk.IconLocation = "$Target,0"
    $lnk.Save()
}

function Get-ShortcutTarget { param([string]$Path) $shell.CreateShortcut($Path).TargetPath }

# A machine as left by updating a per-user 2.9.0 install to the per-machine 2.10.0.
function New-Fixture {
    $id   = [guid]::NewGuid().ToString('N')
    $root = Join-Path ([IO.Path]::GetTempPath()) "cleanup_legacy_test_$id"
    $reg  = "HKCU:\Software\SysmonWidgetTests\$id"

    $fx = [ordered]@{
        Root         = $root
        Reg          = $reg
        Profile      = Join-Path $root 'profile'
        Legacy       = Join-Path $root 'profile\AppData\Local\SysmonWidget'
        Machine      = Join-Path $root 'Program Files\SysmonWidget'
        StartMenu    = Join-Path $root 'profile\Start Menu\Programs'
        Desktop      = Join-Path $root 'profile\Desktop'
        UserKey      = "$reg\Uninstall\SysmonWidget"
        MachineKey   = "$reg\MachineUninstall\SysmonWidget"
        ProductKey   = "$reg\corentingodon\SysmonWidget"
    }
    $fx.LegacyExe  = Join-Path $fx.Legacy  'sysmon-widget.exe'
    $fx.MachineExe = Join-Path $fx.Machine 'sysmon-widget.exe'

    foreach ($f in 'sysmon-widget.exe', 'uninstall.exe', 'hardware\read_temp.ps1', 'hardware\LibreHardwareMonitorLib.dll') {
        $p = Join-Path $fx.Legacy $f
        New-Item -ItemType Directory -Path (Split-Path $p) -Force | Out-Null
        Set-Content -Path $p -Value 'legacy'
    }
    New-Item -ItemType Directory -Path $fx.Machine -Force | Out-Null
    Set-Content -Path $fx.MachineExe -Value 'machine'

    New-Item -Path $fx.UserKey -Force | Out-Null
    Set-ItemProperty -Path $fx.UserKey -Name InstallLocation -Value "`"$($fx.Legacy)`""
    Set-ItemProperty -Path $fx.UserKey -Name Publisher       -Value 'corentingodon'
    Set-ItemProperty -Path $fx.UserKey -Name MainBinaryName  -Value 'sysmon-widget.exe'
    New-Item -Path $fx.ProductKey -Force | Out-Null
    Set-ItemProperty -Path $fx.ProductKey -Name '(default)' -Value $fx.Legacy
    New-Item -Path $fx.MachineKey -Force | Out-Null
    Set-ItemProperty -Path $fx.MachineKey -Name InstallLocation -Value "`"$($fx.Machine)`""

    New-Shortcut (Join-Path $fx.StartMenu 'SysmonWidget.lnk') $fx.LegacyExe
    New-Shortcut (Join-Path $fx.Desktop 'Outils\SysmonWidget.lnk') $fx.LegacyExe
    Set-Content -Path (Join-Path $root 'other.exe') -Value 'other'
    New-Shortcut (Join-Path $fx.Desktop 'Other.lnk') (Join-Path $root 'other.exe')

    [pscustomobject]$fx
}

function Remove-Fixture {
    param($fx)
    Remove-Item -LiteralPath $fx.Reg -Recurse -Force -ErrorAction SilentlyContinue
    Get-ChildItem -LiteralPath $fx.Root -Recurse -Force -Attributes ReparsePoint -ErrorAction SilentlyContinue |
        ForEach-Object { cmd /c rmdir "$($_.FullName)" 2>$null }   # junctions first, without following them
    Remove-Item -LiteralPath $fx.Root -Recurse -Force -ErrorAction SilentlyContinue
}

# Runs the real script on the fixture; returns its exit code.
function Invoke-Cleanup {
    param($fx, [string]$CurrentExe = $fx.MachineExe)
    $out = Join-Path $fx.Root 'run.out'
    $roots = "'$($fx.StartMenu)','$($fx.Desktop)'"
    $cmd = "& '$scriptPath' -CurrentExe '$CurrentExe' -UninstallKey '$($fx.UserKey)' " +
           "-MachineUninstallKey '$($fx.MachineKey)' -ProductKeyRoot '$($fx.Reg)' " +
           "-AllowedRoot '$($fx.Profile)' -ShortcutRoots $roots; exit `$LASTEXITCODE"
    $p = Start-Process powershell -ArgumentList '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-Command', $cmd `
        -WindowStyle Hidden -RedirectStandardOutput $out -PassThru -Wait
    $script:lastOutput = Get-Content $out -Raw -ErrorAction SilentlyContinue
    $p.ExitCode
}

function Assert-LegacyUntouched {
    param($fx)
    Assert-True  (Test-Path -LiteralPath $fx.LegacyExe) 'legacy exe was deleted'
    Assert-True  (Test-Path -LiteralPath $fx.UserKey)   'per-user uninstall entry was deleted'
    Assert-Equal (Get-ShortcutTarget (Join-Path $fx.StartMenu 'SysmonWidget.lnk')) $fx.LegacyExe 'start menu shortcut'
}

# ── Normal migration ─────────────────────────────────────────────────────────

Test-Case 'removes the per-user copy: program files, then the emptied folders' {
    Assert-Equal (Invoke-Cleanup $fx) 0 'exit code'
    Assert-False (Test-Path -LiteralPath $fx.Legacy) "legacy folder still there. Output: $lastOutput"
}

Test-Case 'removes the per-user uninstall entry, so "Apps" lists the app once' {
    Invoke-Cleanup $fx | Out-Null
    Assert-False (Test-Path -LiteralPath $fx.UserKey) 'per-user uninstall entry still there'
}

Test-Case 'removes the per-user install-location key and its now-empty publisher key' {
    Invoke-Cleanup $fx | Out-Null
    Assert-False (Test-Path -LiteralPath $fx.ProductKey) 'product key still there'
    Assert-False (Test-Path -LiteralPath (Split-Path $fx.ProductKey)) 'empty publisher key still there'
}

Test-Case 'keeps a publisher key that still holds another product' {
    New-Item -Path "$($fx.Reg)\corentingodon\OtherApp" -Force | Out-Null
    Invoke-Cleanup $fx | Out-Null
    Assert-False (Test-Path -LiteralPath $fx.ProductKey) 'product key still there'
    Assert-True  (Test-Path -LiteralPath "$($fx.Reg)\corentingodon\OtherApp") 'other product key was deleted'
}

Test-Case 'retargets the start menu shortcut and a shortcut in a desktop subfolder to the new exe' {
    Invoke-Cleanup $fx | Out-Null
    Assert-Equal (Get-ShortcutTarget (Join-Path $fx.StartMenu 'SysmonWidget.lnk'))       $fx.MachineExe 'start menu shortcut'
    Assert-Equal (Get-ShortcutTarget (Join-Path $fx.Desktop 'Outils\SysmonWidget.lnk')) $fx.MachineExe 'desktop\Outils shortcut'
}

Test-Case 'leaves shortcuts to other programs alone' {
    Invoke-Cleanup $fx | Out-Null
    Assert-Equal (Get-ShortcutTarget (Join-Path $fx.Desktop 'Other.lnk')) (Join-Path $fx.Root 'other.exe') 'unrelated shortcut'
}

Test-Case 'keeps files it does not know in the old folder, and the folder with them' {
    Set-Content -Path (Join-Path $fx.Legacy 'notes.txt') -Value 'mine'
    Invoke-Cleanup $fx | Out-Null
    Assert-True  (Test-Path -LiteralPath (Join-Path $fx.Legacy 'notes.txt')) 'unknown file was deleted'
    Assert-False (Test-Path -LiteralPath $fx.LegacyExe) 'legacy exe still there'
}

Test-Case 'accepts the exe path in \\?\ form, as Rust paths can be' {
    Assert-Equal (Invoke-Cleanup $fx -CurrentExe "\\?\$($fx.MachineExe)") 0 'exit code'
    Assert-False (Test-Path -LiteralPath $fx.LegacyExe) "legacy exe still there. Output: $lastOutput"
    # The shortcut must get a normal path: Explorer does not handle \\?\ targets well.
    Assert-Equal (Get-ShortcutTarget (Join-Path $fx.StartMenu 'SysmonWidget.lnk')) $fx.MachineExe 'start menu shortcut'
}

Test-Case 'running it again once everything is clean does nothing and succeeds' {
    Invoke-Cleanup $fx | Out-Null
    Assert-Equal (Invoke-Cleanup $fx) 0 'second run exit code'
}

# ── When to do nothing ───────────────────────────────────────────────────────

Test-Case 'does nothing without a per-user uninstall entry' {
    Remove-Item -LiteralPath $fx.UserKey -Recurse -Force
    Assert-Equal (Invoke-Cleanup $fx) 0 'exit code'
    Assert-True  (Test-Path -LiteralPath $fx.LegacyExe) 'files were deleted without an uninstall entry'
}

Test-Case 'does nothing when not started from the per-machine install (dev build, copied exe)' {
    $dev = Join-Path $fx.Root 'target\release\sysmon-widget.exe'
    New-Item -ItemType Directory -Path (Split-Path $dev) -Force | Out-Null
    Set-Content -Path $dev -Value 'dev'
    Invoke-Cleanup $fx -CurrentExe $dev | Out-Null
    Assert-LegacyUntouched $fx
}

Test-Case 'does nothing when the per-user entry points at the running install itself' {
    Set-ItemProperty -Path $fx.UserKey -Name InstallLocation -Value "`"$($fx.Machine)`""
    Invoke-Cleanup $fx | Out-Null
    Assert-True (Test-Path -LiteralPath $fx.MachineExe) 'running exe was deleted'
    Assert-True (Test-Path -LiteralPath $fx.UserKey)    'uninstall entry was deleted'
}

Test-Case 'deletes nothing outside the user profile (elevated: the entry is user-writable)' {
    # Any user can point InstallLocation at a protected folder; the elevated
    # widget must not become a way to delete files there.
    $outside = Join-Path $fx.Root 'Protected\App'
    New-Item -ItemType Directory -Path $outside -Force | Out-Null
    Set-Content -Path (Join-Path $outside 'uninstall.exe') -Value 'someone else'
    Set-ItemProperty -Path $fx.UserKey -Name InstallLocation -Value "`"$outside`""
    Invoke-Cleanup $fx | Out-Null
    Assert-True (Test-Path -LiteralPath (Join-Path $outside 'uninstall.exe')) 'file outside the profile was deleted'
    Assert-True (Test-Path -LiteralPath $fx.UserKey) 'uninstall entry was deleted'
}

Test-Case 'does not follow a junction from the profile to another folder' {
    $target = Join-Path $fx.Root 'Protected\Victim'
    New-Item -ItemType Directory -Path $target -Force | Out-Null
    Set-Content -Path (Join-Path $target 'uninstall.exe') -Value 'someone else'
    $junction = Join-Path $fx.Profile 'AppData\Local\Trap'
    New-Item -ItemType Junction -Path $junction -Target $target | Out-Null
    Set-ItemProperty -Path $fx.UserKey -Name InstallLocation -Value "`"$junction`""
    Invoke-Cleanup $fx | Out-Null
    Assert-True (Test-Path -LiteralPath (Join-Path $target 'uninstall.exe')) 'file behind the junction was deleted'
}

Test-Case 'ignores a binary name that is a path rather than a file name' {
    $outside = Join-Path $fx.Root 'Protected'
    New-Item -ItemType Directory -Path $outside -Force | Out-Null
    Set-Content -Path (Join-Path $outside 'victim.exe') -Value 'someone else'
    Set-ItemProperty -Path $fx.UserKey -Name MainBinaryName -Value '..\..\..\..\Protected\victim.exe'
    Invoke-Cleanup $fx | Out-Null
    Assert-True (Test-Path -LiteralPath (Join-Path $outside 'victim.exe')) 'file reached through MainBinaryName was deleted'
}

# ── Partial failure ──────────────────────────────────────────────────────────

Test-Case 'keeps the uninstall entry when the old exe cannot be deleted (still running), to retry later' {
    $lock = [IO.File]::Open($fx.LegacyExe, 'Open', 'Read', 'None')
    try {
        $code = Invoke-Cleanup $fx
    } finally {
        $lock.Close()
    }
    Assert-True ($code -ne 0) 'exit code should report the failure'
    Assert-True (Test-Path -LiteralPath $fx.UserKey) 'uninstall entry deleted while the old exe is still there'
    # Shortcuts are fixed regardless: they must not launch the old copy.
    Assert-Equal (Get-ShortcutTarget (Join-Path $fx.StartMenu 'SysmonWidget.lnk')) $fx.MachineExe 'start menu shortcut'
}

Remove-Item -LiteralPath 'HKCU:\Software\SysmonWidgetTests' -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ''
if ($script:failures -eq 0) { Write-Host 'All tests passed.' } else { Write-Host "$($script:failures) test(s) failed." }
exit $script:failures
