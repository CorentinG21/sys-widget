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

function Read-GpuSensors {
    param($hw)
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

    return [ordered]@{
        usage         = $usage
        temp          = $temp
        vram_used_mb  = $vramUsed
        vram_total_mb = $vramTotal
    }
}

function Get-GpuData {
    param($hardware)

    # Cherche dans les hardware de premier niveau ET dans SubHardware (iGPU AMD/Intel)
    foreach ($hw in $hardware) {
        if ($hw.HardwareType.ToString() -match 'Gpu') {
            $hw.Update()
            return Read-GpuSensors $hw
        }
        foreach ($sub in $hw.SubHardware) {
            if ($sub.HardwareType.ToString() -match 'Gpu') {
                $sub.Update()
                return Read-GpuSensors $sub
            }
        }
    }
    return $null
}

function Get-NvidiaFallback {
    # Fallback pour les GPU NVIDIA que LHM ne detecte pas.
    # nvidia-smi est installe avec les drivers NVIDIA (dans PATH ou NVSMI/).
    $smiPaths = @(
        'nvidia-smi',
        'C:\Program Files\NVIDIA Corporation\NVSMI\nvidia-smi.exe',
        'C:\Windows\System32\nvidia-smi.exe'
    )
    foreach ($smi in $smiPaths) {
        try {
            $out = & $smi --query-gpu=utilization.gpu,temperature.gpu,memory.used,memory.total --format=csv,noheader,nounits 2>$null
            if ($LASTEXITCODE -eq 0 -and $out) {
                $parts = ($out.Trim() -split ',\s*')
                if ($parts.Count -ge 4) {
                    return [ordered]@{
                        usage         = [float]$parts[0]
                        temp          = [float]$parts[1]
                        vram_used_mb  = [float]$parts[2]
                        vram_total_mb = [float]$parts[3]
                    }
                }
            }
        } catch {}
    }
    return $null
}

try {
    while ($true) {
        $gpuData = Get-GpuData $c.Hardware
        # LHM ne detecte pas les GPU NVIDIA sur certains systemes -> fallback nvidia-smi
        if ($null -eq $gpuData) {
            $gpuData = Get-NvidiaFallback
        }

        $obj = [ordered]@{
            cpu_temp = Get-CpuTemp $c.Hardware
            gpu      = $gpuData
        }
        [Console]::WriteLine(($obj | ConvertTo-Json -Compress -Depth 3))
        [Console]::Out.Flush()
        Start-Sleep -Milliseconds 2000
    }
} finally {
    $c.Close()
}
