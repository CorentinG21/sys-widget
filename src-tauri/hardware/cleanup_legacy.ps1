# Removes a per-user install (<= 2.9.0, AppData\Local) left behind after the
# move to a per-machine install (>= 2.10.0, Program Files).
#
# The NSIS updater installs the new version next to the old one without
# touching it: the old copy keeps its "Apps" entry, and the user's shortcuts
# keep launching it - elevated, from a folder any program of the user can
# modify. Run at every start of the per-machine build; does nothing once clean.
#
# This runs elevated but reads values the user can write (HKCU): it only ever
# deletes known file names, only inside the user's profile, never through a
# junction, and never recursively.
#
# Exit codes: 0 done or nothing to do, 1 skipped (unsafe or unusable entry),
# 2 partly done (old exe still there, probably running): retried next start.

param(
    [Parameter(Mandatory = $true)] [string]$CurrentExe,
    [string]$UninstallKey        = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\SysmonWidget',
    [string]$MachineUninstallKey = 'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\SysmonWidget',
    [string]$ProductKeyRoot      = 'HKCU:\Software',
    [string]$AllowedRoot         = [Environment]::GetFolderPath('UserProfile'),
    [string[]]$ShortcutRoots     = @(
        [Environment]::GetFolderPath('Programs'),
        [Environment]::GetFolderPath('Desktop'),
        [Environment]::GetFolderPath('Startup'),
        (Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar')
    )
)

$ProductName = 'SysmonWidget'

function Write-Log { param([string]$Message) [Console]::Out.WriteLine($Message) }

function Normalize-Path {
    param([string]$Path)
    if (-not $Path) { return $null }
    # Rust can hand over verbatim paths (\\?\C:\...).
    $p = $Path.Trim().Trim('"') -replace '^\\\\\?\\(?=[A-Za-z]:)', ''
    $p = $p.TrimEnd('\')
    if (-not $p -or -not [IO.Path]::IsPathRooted($p)) { return $null }
    try { return [IO.Path]::GetFullPath($p) } catch { return $null }
}

function Test-Within {
    param([string]$Path, [string]$Root)
    $Path.StartsWith($Root.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)
}

# True when $Dir or one of its parents below $Root is a junction or symlink:
# deleting through it could reach a folder outside the profile.
function Test-ReparseOnPath {
    param([string]$Dir, [string]$Root)
    $root = $Root.TrimEnd('\')
    $d = $Dir
    while ($d -and $d.Length -gt $root.Length) {
        $item = Get-Item -LiteralPath $d -Force -ErrorAction SilentlyContinue
        if ($item -and ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { return $true }
        $d = Split-Path -Parent $d
    }
    return $false
}

function Remove-EmptyKey {
    param([string]$Key)
    $k = Get-Item -LiteralPath $Key -ErrorAction SilentlyContinue
    if ($k -and $k.SubKeyCount -eq 0 -and $k.ValueCount -eq 0) {
        Remove-Item -LiteralPath $Key -Force -ErrorAction SilentlyContinue
    }
}

# ── Is there anything to do, and is this the per-machine install? ──────────

$entry = Get-ItemProperty -LiteralPath $UninstallKey -ErrorAction SilentlyContinue
if (-not $entry) { exit 0 }

$CurrentExe = Normalize-Path $CurrentExe
$currentDir = if ($CurrentExe) { Split-Path -Parent $CurrentExe } else { $null }
$machine    = Get-ItemProperty -LiteralPath $MachineUninstallKey -ErrorAction SilentlyContinue
$machineDir = if ($machine) { Normalize-Path $machine.InstallLocation } else { $null }
if (-not $currentDir -or $currentDir -ne $machineDir) {
    # Dev build or a copied exe: shortcuts must not be pointed at it.
    Write-Log "not running from the per-machine install ($currentDir), nothing done"
    exit 0
}

$legacyDir = Normalize-Path $entry.InstallLocation
if (-not $legacyDir) {
    Write-Log "per-user entry has no usable InstallLocation, nothing done"
    exit 1
}
if ($legacyDir -eq $currentDir) { exit 0 }

$binary = if ($entry.MainBinaryName) { [string]$entry.MainBinaryName } else { 'sysmon-widget.exe' }
if ($binary -ne [IO.Path]::GetFileName($binary)) {
    Write-Log "per-user entry has an invalid MainBinaryName ($binary), nothing done"
    exit 1
}

$allowed = Normalize-Path $AllowedRoot
if (-not $allowed -or -not (Test-Within $legacyDir $allowed)) {
    Write-Log "per-user install $legacyDir is outside $allowed, nothing done"
    exit 1
}
if (Test-ReparseOnPath $legacyDir $allowed) {
    Write-Log "per-user install $legacyDir goes through a junction or symlink, nothing done"
    exit 1
}

$legacyExe = Join-Path $legacyDir $binary
Write-Log "removing per-user install $legacyDir"

# ── Shortcuts: point them at this install before the old exe disappears ────

$shell = New-Object -ComObject WScript.Shell
foreach ($root in $ShortcutRoots) {
    if (-not $root -or -not (Test-Path -LiteralPath $root)) { continue }
    Get-ChildItem -LiteralPath $root -Filter *.lnk -Recurse -File -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            $lnk = $shell.CreateShortcut($_.FullName)
            if ($lnk.TargetPath -ne $legacyExe) { return }
            $lnk.TargetPath       = $CurrentExe
            $lnk.WorkingDirectory = $currentDir
            if ($lnk.IconLocation -like "$legacyExe*") { $lnk.IconLocation = "$CurrentExe,0" }
            $lnk.Save()
            Write-Log "shortcut now opens $CurrentExe : $($_.FullName)"
        } catch {
            Write-Log "shortcut left as is ($($_.Exception.Message)): $($_.FullName)"
        }
    }
}

# ── Files: only what the old installer put there ───────────────────────────

$files = $binary, 'uninstall.exe', 'hardware\read_temp.ps1', 'hardware\LibreHardwareMonitorLib.dll'
foreach ($f in $files) {
    $path = Join-Path $legacyDir $f
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { continue }
    try {
        Remove-Item -LiteralPath $path -Force -ErrorAction Stop
    } catch {
        Write-Log "could not delete $path : $($_.Exception.Message)"
    }
}
foreach ($dir in (Join-Path $legacyDir 'hardware'), $legacyDir) {
    if ((Test-Path -LiteralPath $dir -PathType Container) -and -not (Get-ChildItem -LiteralPath $dir -Force)) {
        Remove-Item -LiteralPath $dir -Force -ErrorAction SilentlyContinue
    }
}

if (Test-Path -LiteralPath $legacyExe) {
    Write-Log "old exe still present (running?): uninstall entry kept, retried at next start"
    exit 2
}

# ── Registry: the old "Apps" entry and install-location key ────────────────

Remove-Item -LiteralPath $UninstallKey -Recurse -Force -ErrorAction SilentlyContinue
$publisher = [string]$entry.Publisher
if ($publisher -and $publisher -eq [IO.Path]::GetFileName($publisher)) {
    $publisherKey = Join-Path $ProductKeyRoot $publisher
    Remove-Item -LiteralPath (Join-Path $publisherKey $ProductName) -Recurse -Force -ErrorAction SilentlyContinue
    Remove-EmptyKey $publisherKey
}

Write-Log "per-user install removed"
exit 0
