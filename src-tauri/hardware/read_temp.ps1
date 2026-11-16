param(
    # PID du processus qui a lance ce script (le widget). Quand il disparait,
    # le script s'arrete : rien d'autre ne stoppe cette boucle si le widget
    # plante, est tue ou est remplace par une mise a jour.
    [int]$ParentPid = 0
)

$parent = $null
if ($ParentPid -gt 0) {
    try {
        # On garde un handle sur le processus : pas de confusion possible si
        # Windows reattribue plus tard le meme PID a un autre programme.
        $parent = [System.Diagnostics.Process]::GetProcessById($ParentPid)
    } catch {
        exit 0   # deja termine
    }
}

function Test-ProcessGone {
    param($process)
    if ($null -eq $process) { return $false }
    try {
        return $process.HasExited
    } catch {
        return $null -eq (Get-Process -Id $process.Id -ErrorAction SilentlyContinue)
    }
}

# LibreHardwareMonitor est optionnel : s'il ne se charge pas, la boucle
# continue avec les fallbacks (nvidia-smi, compteurs Windows, WMI).
$c = $null
try {
    # Tauri lance le script avec un chemin "verbatim" (\\?\C:\...), que
    # .NET Framework ne sait pas charger : on retire le prefixe.
    $scriptDir = $PSScriptRoot -replace '^\\\\\?\\(?=[A-Za-z]:)', ''
    Add-Type -Path (Join-Path $scriptDir 'LibreHardwareMonitorLib.dll') -ErrorAction Stop
    $c = New-Object LibreHardwareMonitor.Hardware.Computer -ErrorAction Stop
    $c.IsCpuEnabled = $true
    $c.IsGpuEnabled = $true
    $c.Open()
} catch {
    $c = $null
    $reason = $_.Exception.Message
    if ($_.Exception.LoaderExceptions) { $reason = $_.Exception.LoaderExceptions[0].Message }
    [Console]::Error.WriteLine("LibreHardwareMonitor indisponible, fallbacks uniquement : $reason")
}

function Get-CpuTemp {
    param($hardware)

    # Uniquement le materiel CPU : un GPU expose une sonde "GPU Core", que le
    # motif de la passe 2 prendrait pour la temperature du processeur.
    $nodes = @()
    foreach ($hw in $hardware) {
        if ($hw.HardwareType.ToString() -ne 'Cpu') { continue }
        $hw.Update()
        $nodes += $hw
        foreach ($sub in $hw.SubHardware) {
            $sub.Update()
            $nodes += $sub
        }
    }

    # Passe 1 : sondes privilegiees (AMD Tctl/Tdie, Intel CPU Package).
    # Passe 2 : n'importe quelle sonde CPU/Core.
    foreach ($pattern in 'Tctl|Tdie|CPU Package', 'CPU|Core') {
        foreach ($node in $nodes) {
            foreach ($s in $node.Sensors) {
                # Une sonde a 0 n'est pas lisible (pas de droits admin ou pas de
                # pilote PawnIO) : on passe a la suivante, puis au fallback WMI.
                if ($s.SensorType.ToString() -ne 'Temperature' -or $null -eq $s.Value -or $s.Value -le 0) { continue }
                # "Distance to TjMax" est une marge, pas une temperature.
                if ($s.Name -match $pattern -and $s.Name -notmatch 'Distance') { return $s.Value }
            }
        }
    }
    return $null
}

function Read-GpuSensors {
    param($hw)
    $coreLoad  = $null
    $d3dLoad   = $null
    $firstLoad = $null
    $temp      = $null
    $vramUsed  = $null
    $vramTotal = $null
    $d3dUsed   = $null
    $d3dTotal  = $null

    foreach ($s in $hw.Sensors) {
        if ($null -eq $s.Value) { continue }
        $type = $s.SensorType.ToString()
        $name = $s.Name

        # Noms exacts : tous les capteurs "D3D ..." contiennent "3D", et
        # "D3D Shared Memory Used" (RAM systeme pretee au GPU) contient
        # "Memory Used". Avec un motif partiel, le dernier capteur liste gagnait.
        if ($type -eq 'Load') {
            if     ($name -eq 'GPU Core')  { $coreLoad = $s.Value }
            elseif ($name -eq 'D3D 3D')    { $d3dLoad  = $s.Value }
            elseif ($null -eq $firstLoad)  { $firstLoad = $s.Value }
        }
        if ($type -eq 'Temperature' -and $null -eq $temp) {
            $temp = $s.Value
        }
        if ($type -eq 'SmallData') {
            switch ($name) {
                'GPU Memory Used'            { $vramUsed  = $s.Value }
                'GPU Memory Total'           { $vramTotal = $s.Value }
                'D3D Dedicated Memory Used'  { $d3dUsed   = $s.Value }
                'D3D Dedicated Memory Total' { $d3dTotal  = $s.Value }
            }
        }
    }

    $usage = $coreLoad
    if ($null -eq $usage) { $usage = $d3dLoad }
    if ($null -eq $usage) { $usage = $firstLoad }
    if ($null -eq $vramUsed)  { $vramUsed  = $d3dUsed }
    if ($null -eq $vramTotal) { $vramTotal = $d3dTotal }

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
        if (Test-ProcessGone $parent) { break }

        $hardware = @()
        if ($c) { $hardware = $c.Hardware }

        $gpuData = Get-GpuData $hardware
        # Fallback 1 : nvidia-smi (NVIDIA non detecte par LHM)
        if ($null -eq $gpuData) {
            $gpuData = Get-NvidiaFallback
        }
        # Fallback 2 : Windows Perf Counters (AMD iGPU, Intel, tout GPU)
        if ($null -eq $gpuData) {
            $gpuData = Get-WindowsGpuFallback
        }

        $cpuTemp = Get-CpuTemp $hardware
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
    if ($c) { $c.Close() }
}
