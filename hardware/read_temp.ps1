Add-Type -Path "$PSScriptRoot\LibreHardwareMonitorLib.dll"

$c = New-Object LibreHardwareMonitor.Hardware.Computer
$c.IsCpuEnabled = $true
$c.IsGpuEnabled = $true
$c.Open()

function Get-CpuTemp {
    param($hardware)
    $temp = $null
    foreach ($hw in $hardware) {
        $hw.Update()
        foreach ($s in $hw.Sensors) {
            if ($s.SensorType.ToString() -eq 'Temperature' -and $null -ne $s.Value) {
                if ($s.Name -match 'Tctl|CPU Package') { $temp = $s.Value; break }
            }
        }
        foreach ($sub in $hw.SubHardware) {
            $sub.Update()
            foreach ($s in $sub.Sensors) {
                if ($s.SensorType.ToString() -eq 'Temperature' -and $null -ne $s.Value) {
                    if ($s.Name -match 'Tctl|CPU Package') { $temp = $s.Value; break }
                }
            }
            if ($null -ne $temp) { break }
        }
        if ($null -ne $temp) { break }
    }
    if ($null -eq $temp) {
        foreach ($hw in $hardware) {
            foreach ($s in $hw.Sensors) {
                if ($s.SensorType.ToString() -eq 'Temperature' -and $null -ne $s.Value) {
                    if ($s.Name -match 'CPU|Core') { $temp = $s.Value; break }
                }
            }
            if ($null -ne $temp) { break }
        }
    }
    return $temp
}

function Get-GpuData {
    param($hardware)
    foreach ($hw in $hardware) {
        if ($hw.HardwareType.ToString() -notmatch 'Gpu') { continue }
        $hw.Update()

        $usage     = $null
        $firstLoad = $null
        $temp      = $null
        $vramUsed  = $null
        $vramTotal = $null

        foreach ($s in $hw.Sensors) {
            if ($null -eq $s.Value) { continue }
            $type = $s.SensorType.ToString()
            $name = $s.Name

            if ($type -eq 'Load') {
                if ($name -match 'GPU Core|3D') {
                    $usage = $s.Value
                } elseif ($null -eq $firstLoad) {
                    $firstLoad = $s.Value
                }
            }
            if ($type -eq 'Temperature' -and $null -eq $temp) {
                $temp = $s.Value
            }
            if ($type -eq 'SmallData') {
                if ($name -match 'Memory Used') { $vramUsed  = $s.Value }
                if ($name -match 'Memory Total') { $vramTotal = $s.Value }
            }
        }

        if ($null -eq $usage) { $usage = $firstLoad }

        if ($null -ne $usage -or $null -ne $temp) {
            return [ordered]@{
                usage          = $usage
                temp           = $temp
                vram_used_mb   = $vramUsed
                vram_total_mb  = $vramTotal
            }
        }
    }
    return $null
}

try {
    while ($true) {
        $obj = [ordered]@{
            cpu_temp = Get-CpuTemp $c.Hardware
            gpu      = Get-GpuData $c.Hardware
        }
        [Console]::WriteLine(($obj | ConvertTo-Json -Compress -Depth 3))
        [Console]::Out.Flush()
        Start-Sleep -Milliseconds 2000
    }
} finally {
    $c.Close()
}
