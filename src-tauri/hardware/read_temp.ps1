Add-Type -Path "$PSScriptRoot\LibreHardwareMonitorLib.dll"

$c = New-Object LibreHardwareMonitor.Hardware.Computer
$c.IsCpuEnabled = $true
$c.IsGpuEnabled = $true
$c.Open()

function Get-CpuTemp {
    param($hardware)
    $temp = $null
    # Pass 1 : sondes privilegiees (AMD Tctl/Tdie, Intel CPU Package)
    foreach ($hw in $hardware) {
        $hw.Update()
        foreach ($s in $hw.Sensors) {
            if ($s.SensorType.ToString() -eq 'Temperature' -and $null -ne $s.Value) {
                if ($s.Name -match 'Tctl|Tdie|CPU Package') { $temp = $s.Value; break }
            }
        }
        if ($null -ne $temp) { break }
        foreach ($sub in $hw.SubHardware) {
            $sub.Update()
            foreach ($s in $sub.Sensors) {
                if ($s.SensorType.ToString() -eq 'Temperature' -and $null -ne $s.Value) {
                    if ($s.Name -match 'Tctl|Tdie|CPU Package') { $temp = $s.Value; break }
                }
            }
            if ($null -ne $temp) { break }
        }
        if ($null -ne $temp) { break }
    }
    # Pass 2 : n'importe quelle sonde CPU/Core (top-level ET SubHardware)
    if ($null -eq $temp) {
        foreach ($hw in $hardware) {
            foreach ($s in $hw.Sensors) {
                if ($s.SensorType.ToString() -eq 'Temperature' -and $null -ne $s.Value) {
                    if ($s.Name -match 'CPU|Core') { $temp = $s.Value; break }
                }
            }
            if ($null -ne $temp) { break }
            foreach ($sub in $hw.SubHardware) {
                foreach ($s in $sub.Sensors) {
                    if ($s.SensorType.ToString() -eq 'Temperature' -and $null -ne $s.Value) {
                        if ($s.Name -match 'CPU|Core') { $temp = $s.Value; break }
                    }
                }
                if ($null -ne $temp) { break }
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

function Get-CpuTempWmiFallback {
    # Fallback via WMI ThermalZone — fonctionne sans droits admin sur la plupart des laptops.
    # Renvoie la temperature en Celsius, ou $null si indisponible.
    try {
        $zones = Get-CimInstance -Namespace 'root\wmi' -ClassName 'MSAcpi_ThermalZoneTemperature' -ErrorAction SilentlyContinue
        if ($zones) {
            $temp = ($zones | Measure-Object -Property CurrentTemperature -Maximum).Maximum
            if ($null -ne $temp -and $temp -gt 2731) {
                return [math]::Round($temp / 10 - 273.15, 1)
            }
        }
    } catch {}
    return $null
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

function Get-WindowsGpuFallback {
    # Dernier recours : Windows Performance Counters (memes donnees que le Gestionnaire des taches).
    # Fonctionne pour TOUS les GPU : AMD, Intel iGPU, NVIDIA — sans outil tiers.
    try {
        # Charge GPU 3D (toutes les instances = tous les processus cumules)
        $eng = Get-Counter '\GPU Engine(*engtype_3D*)\Utilization Percentage' -ErrorAction SilentlyContinue
        if ($null -eq $eng) { return $null }

        $usage = [math]::Min(100, [float]($eng.CounterSamples |
            Where-Object { $_.CookedValue -ge 0 } |
            Measure-Object -Property CookedValue -Sum).Sum)

        # VRAM dedieee utilisee (MB)
        $memUsed = Get-Counter '\GPU Adapter Memory(*)\Dedicated Usage' -ErrorAction SilentlyContinue
        $vramUsedMb = if ($memUsed) {
            [float]([math]::Round(($memUsed.CounterSamples |
                Measure-Object -Property CookedValue -Sum).Sum / 1MB, 1))
        } else { $null }

        # VRAM totale via WMI (AdapterRAM en bytes, 0 pour iGPU memoire partagee)
        $vga = Get-CimInstance Win32_VideoController -ErrorAction SilentlyContinue |
               Where-Object { $_.CurrentHorizontalResolution -gt 0 } |
               Select-Object -First 1
        $vramTotalMb = if ($vga -and $vga.AdapterRAM -gt 0) {
            [float]([math]::Round($vga.AdapterRAM / 1MB, 1))
        } else { $null }

        return [ordered]@{
            usage         = $usage
            temp          = $null   # non disponible sans API vendeur
            vram_used_mb  = $vramUsedMb
            vram_total_mb = $vramTotalMb
        }
    } catch { return $null }
}

try {
    while ($true) {
        $gpuData = Get-GpuData $c.Hardware
        # Fallback 1 : nvidia-smi (NVIDIA non detecte par LHM)
        if ($null -eq $gpuData) {
            $gpuData = Get-NvidiaFallback
        }
        # Fallback 2 : Windows Perf Counters (AMD iGPU, Intel, tout GPU)
        if ($null -eq $gpuData) {
            $gpuData = Get-WindowsGpuFallback
        }

        $cpuTemp = Get-CpuTemp $c.Hardware
        if ($null -eq $cpuTemp) { $cpuTemp = Get-CpuTempWmiFallback }

        $obj = [ordered]@{
            cpu_temp = $cpuTemp
            gpu      = $gpuData
        }
        [Console]::WriteLine(($obj | ConvertTo-Json -Compress -Depth 3))
        [Console]::Out.Flush()
        Start-Sleep -Milliseconds 2000
    }
} finally {
    $c.Close()
}
