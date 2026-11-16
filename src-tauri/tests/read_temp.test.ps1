# Tests for hardware/read_temp.ps1. Plain Windows PowerShell 5.1, no Pester.
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File src-tauri\tests\read_temp.test.ps1
#
# Exit code = number of failed tests.
#
# Part 1 runs the script's sensor-selection functions against fake hardware
# shaped like LibreHardwareMonitor's (HardwareType / Sensors / SubHardware).
# Part 2 launches the real script and checks how it behaves as a process.

$scriptPath = Join-Path $PSScriptRoot '..\hardware\read_temp.ps1' | Resolve-Path | Select-Object -ExpandProperty Path
$script:failures = 0

function Test-Case {
    param([string]$Name, [scriptblock]$Body)
    try {
        & $Body
        Write-Host "PASS  $Name"
    } catch {
        $script:failures++
        Write-Host "FAIL  $Name"
        Write-Host "      $($_.Exception.Message)"
    }
}

function Assert-Equal {
    param($Actual, $Expected)
    if ($null -eq $Expected) {
        if ($null -ne $Actual) { throw "expected `$null, got '$Actual'" }
    } elseif ($Actual -ne $Expected) {
        throw "expected '$Expected', got '$Actual'"
    }
}

# ── Part 1: sensor selection ─────────────────────────────────────────────────

# Define the script's functions here without running its main loop.
$ast = [System.Management.Automation.Language.Parser]::ParseFile($scriptPath, [ref]$null, [ref]$null)
$ast.FindAll({ $args[0] -is [System.Management.Automation.Language.FunctionDefinitionAst] }, $false) |
    ForEach-Object { . ([scriptblock]::Create($_.Extent.Text)) }

function New-Sensor {
    param($Type, $Name, $Value)
    [pscustomobject]@{ SensorType = $Type; Name = $Name; Value = $Value }
}

function New-Hardware {
    param($Type, $Sensors = @(), $SubHardware = @())
    $hw = [pscustomobject]@{ HardwareType = $Type; Sensors = @($Sensors); SubHardware = @($SubHardware) }
    $hw | Add-Member -MemberType ScriptMethod -Name Update -Value { } -PassThru
}

Test-Case 'CPU temp: the package probe wins over a core probe' {
    $cpu = New-Hardware 'Cpu' @(
        (New-Sensor 'Temperature' 'CPU Core #1' 50),
        (New-Sensor 'Temperature' 'CPU Package' 60)
    )
    Assert-Equal (Get-CpuTemp @($cpu)) 60
}

Test-Case 'CPU temp: falls back to a core probe when there is no package probe' {
    $cpu = New-Hardware 'Cpu' @(
        (New-Sensor 'Load'        'CPU Total'   12),
        (New-Sensor 'Temperature' 'CPU Core #1' 51)
    )
    Assert-Equal (Get-CpuTemp @($cpu)) 51
}

Test-Case 'CPU temp: reads probes exposed by a sub-hardware of the CPU' {
    $sub = New-Hardware 'Cpu' @((New-Sensor 'Temperature' 'Core (Tctl/Tdie)' 64))
    $cpu = New-Hardware 'Cpu' @() @($sub)
    Assert-Equal (Get-CpuTemp @($cpu)) 64
}

Test-Case 'CPU temp: never reports the GPU temperature when the CPU has no probe' {
    $cpu = New-Hardware 'Cpu' @((New-Sensor 'Load' 'CPU Total' 12))
    $gpu = New-Hardware 'GpuNvidia' @((New-Sensor 'Temperature' 'GPU Core' 70))
    Assert-Equal (Get-CpuTemp @($cpu, $gpu)) $null
}

Test-Case 'CPU temp: "Distance to TjMax" is a margin, not a temperature' {
    $cpu = New-Hardware 'Cpu' @(
        (New-Sensor 'Temperature' 'CPU Core #1 Distance to TjMax' 45),
        (New-Sensor 'Temperature' 'CPU Core #1'                   55)
    )
    Assert-Equal (Get-CpuTemp @($cpu)) 55
}

Test-Case 'CPU temp: probes without a value are skipped' {
    $cpu = New-Hardware 'Cpu' @(
        (New-Sensor 'Temperature' 'CPU Package' $null),
        (New-Sensor 'Temperature' 'CPU Core #1' 48)
    )
    Assert-Equal (Get-CpuTemp @($cpu)) 48
}

Test-Case 'CPU temp: a probe reading 0 means "no access" (not elevated, no driver), not 0 degrees' {
    # Observed on a Ryzen 5 3600 without admin rights: "Core (Tctl/Tdie)" = 0.
    $cpu = New-Hardware 'Cpu' @(
        (New-Sensor 'Load'        'CPU Total'        9),
        (New-Sensor 'Temperature' 'Core (Tctl/Tdie)' 0)
    )
    Assert-Equal (Get-CpuTemp @($cpu)) $null
}

Test-Case 'CPU temp: a zero package probe does not hide a valid core probe' {
    $cpu = New-Hardware 'Cpu' @(
        (New-Sensor 'Temperature' 'CPU Package' 0),
        (New-Sensor 'Temperature' 'CPU Core #1' 47)
    )
    Assert-Equal (Get-CpuTemp @($cpu)) 47
}

Test-Case 'CPU temp: no hardware at all gives $null' {
    Assert-Equal (Get-CpuTemp @()) $null
}

Test-Case 'GPU VRAM: "GPU Memory Used" is not overwritten by "D3D Shared Memory Used"' {
    $gpu = New-Hardware 'GpuNvidia' @(
        (New-Sensor 'SmallData' 'GPU Memory Used'           2048),
        (New-Sensor 'SmallData' 'GPU Memory Total'          8192),
        (New-Sensor 'SmallData' 'D3D Dedicated Memory Used' 1900),
        (New-Sensor 'SmallData' 'D3D Shared Memory Used'     150)
    )
    $r = Read-GpuSensors $gpu
    Assert-Equal $r.vram_used_mb  2048
    Assert-Equal $r.vram_total_mb 8192
}

Test-Case 'GPU VRAM: falls back to D3D dedicated memory, never to shared memory' {
    $gpu = New-Hardware 'GpuIntel' @(
        (New-Sensor 'SmallData' 'D3D Dedicated Memory Used' 900),
        (New-Sensor 'SmallData' 'D3D Shared Memory Used'    300)
    )
    Assert-Equal (Read-GpuSensors $gpu).vram_used_mb 900
}

Test-Case 'GPU load: "GPU Core" wins over the per-engine D3D loads listed after it' {
    $gpu = New-Hardware 'GpuNvidia' @(
        (New-Sensor 'Load' 'GPU Core'         35),
        (New-Sensor 'Load' 'D3D 3D'           80),
        (New-Sensor 'Load' 'D3D Video Decode'  5)
    )
    Assert-Equal (Read-GpuSensors $gpu).usage 35
}

Test-Case 'GPU load: without "GPU Core", the 3D engine is used, not the last D3D engine' {
    $gpu = New-Hardware 'GpuIntel' @(
        (New-Sensor 'Load' 'D3D 3D'           80),
        (New-Sensor 'Load' 'D3D Video Decode'  5)
    )
    Assert-Equal (Read-GpuSensors $gpu).usage 80
}

Test-Case 'GPU load: any load probe is better than none' {
    $gpu = New-Hardware 'GpuAmd' @((New-Sensor 'Load' 'GPU Memory Controller' 9))
    Assert-Equal (Read-GpuSensors $gpu).usage 9
}

Test-Case 'GPU temp: first temperature probe' {
    $gpu = New-Hardware 'GpuNvidia' @(
        (New-Sensor 'Temperature' 'GPU Core'     61),
        (New-Sensor 'Temperature' 'GPU Hot Spot' 72)
    )
    Assert-Equal (Read-GpuSensors $gpu).temp 61
}

# ── Part 2: the script as a process ──────────────────────────────────────────

function Start-Hidden {
    param([string[]]$Arguments, [string]$StdOut, [string]$StdErr)
    $params = @{ FilePath = 'powershell'; ArgumentList = $Arguments; WindowStyle = 'Hidden'; PassThru = $true }
    if ($StdOut) { $params.RedirectStandardOutput = $StdOut }
    if ($StdErr) { $params.RedirectStandardError  = $StdErr }
    Start-Process @params
}

function Start-SensorScript {
    param([int]$ParentPid, [string]$StdOut, [string]$StdErr)
    Start-Hidden -StdOut $StdOut -StdErr $StdErr -Arguments @(
        '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass',
        '-File', "`"$scriptPath`"", '-ParentPid', $ParentPid
    )
}

function Wait-FirstLine {
    param([string]$Path, [int]$TimeoutSec = 40)
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $line = Get-Content $Path -TotalCount 1 -ErrorAction SilentlyContinue
        if ($line) { return $line }
        Start-Sleep -Milliseconds 250
    }
    return $null
}

function Stop-Quietly {
    param($Process)
    if ($Process -and -not $Process.HasExited) { Stop-Process -Id $Process.Id -Force -ErrorAction SilentlyContinue }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("read_temp_test_" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmp | Out-Null

Test-Case 'process: emits one JSON object per line with cpu_temp and gpu' {
    $parent = Start-Hidden -Arguments @('-NoProfile', '-Command', 'Start-Sleep 120')
    $child  = $null
    try {
        $out   = Join-Path $tmp 'json.out'
        $child = Start-SensorScript -ParentPid $parent.Id -StdOut $out -StdErr (Join-Path $tmp 'json.err')
        $line  = Wait-FirstLine $out
        if (-not $line) { throw 'no output within 40 s' }
        $names = ($line | ConvertFrom-Json).PSObject.Properties.Name -join ','
        Assert-Equal $names 'cpu_temp,gpu'
    } finally {
        Stop-Quietly $child
        Stop-Quietly $parent
    }
}

Test-Case 'process: loads LibreHardwareMonitor in Windows PowerShell 5.1' {
    # The bundled library must be a .NET Framework build: a .NET 8/9/10 build
    # fails to load here and the script silently runs on fallbacks only.
    $parent = Start-Hidden -Arguments @('-NoProfile', '-Command', 'Start-Sleep 120')
    $child  = $null
    try {
        $out   = Join-Path $tmp 'lhm.out'
        $err   = Join-Path $tmp 'lhm.err'
        $child = Start-SensorScript -ParentPid $parent.Id -StdOut $out -StdErr $err
        if (-not (Wait-FirstLine $out)) { throw 'no output within 40 s' }
        $errors = Get-Content $err -Raw -ErrorAction SilentlyContinue
        if ($errors -match 'LibreHardwareMonitor') { throw $errors.Trim() }
    } finally {
        Stop-Quietly $child
        Stop-Quietly $parent
    }
}

Test-Case 'process: finds the sensor library when launched through a \\?\ path, as the app does' {
    # Tauri hands over a verbatim path (\\?\C:\...), which .NET Framework cannot load from.
    $parent = Start-Hidden -Arguments @('-NoProfile', '-Command', 'Start-Sleep 120')
    $child  = $null
    try {
        $out   = Join-Path $tmp 'verbatim.out'
        $err   = Join-Path $tmp 'verbatim.err'
        $child = Start-Hidden -StdOut $out -StdErr $err -Arguments @(
            '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass',
            '-File', "`"\\?\$scriptPath`"", '-ParentPid', $parent.Id
        )
        if (-not (Wait-FirstLine $out)) { throw 'no output within 40 s' }
        $errors = Get-Content $err -Raw -ErrorAction SilentlyContinue
        if ($errors -match 'file://') { throw "library file not found: $($errors.Trim())" }
    } finally {
        Stop-Quietly $child
        Stop-Quietly $parent
    }
}

Test-Case 'process: stops by itself once the process that launched it is gone' {
    $parent = Start-Hidden -Arguments @('-NoProfile', '-Command', 'Start-Sleep 120')
    $child  = $null
    try {
        $out   = Join-Path $tmp 'orphan.out'
        $child = Start-SensorScript -ParentPid $parent.Id -StdOut $out -StdErr (Join-Path $tmp 'orphan.err')
        if (-not (Wait-FirstLine $out)) { throw 'no output within 40 s' }

        Stop-Process -Id $parent.Id -Force
        if (-not $child.WaitForExit(15000)) {
            throw 'still running 15 s after its parent was killed (orphan)'
        }
    } finally {
        Stop-Quietly $child
        Stop-Quietly $parent
    }
}

Test-Case 'process: does not start its loop when the parent is already gone' {
    $gone = Start-Hidden -Arguments @('-NoProfile', '-Command', 'exit')
    $gone.WaitForExit()
    $child = $null
    try {
        $child = Start-SensorScript -ParentPid $gone.Id -StdOut (Join-Path $tmp 'gone.out') -StdErr (Join-Path $tmp 'gone.err')
        if (-not $child.WaitForExit(15000)) { throw 'still running 15 s after start' }
    } finally {
        Stop-Quietly $child
    }
}

Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ''
if ($script:failures -eq 0) { Write-Host 'All tests passed.' } else { Write-Host "$($script:failures) test(s) failed." }
exit $script:failures
