Add-Type -Path "$PSScriptRoot\LibreHardwareMonitorLib.dll"

$c = New-Object LibreHardwareMonitor.Hardware.Computer
$c.IsCpuEnabled = $true
$c.Open()

try {
    while ($true) {
        $temp = $null

        foreach ($hw in $c.Hardware) {
            $hw.Update()
            foreach ($s in $hw.Sensors) {
                if ($s.SensorType.ToString() -eq 'Temperature' -and $null -ne $s.Value) {
                    if ($s.Name -match 'Tctl|CPU Package') {
                        $temp = $s.Value
                        break
                    }
                }
            }
            foreach ($sub in $hw.SubHardware) {
                $sub.Update()
                foreach ($s in $sub.Sensors) {
                    if ($s.SensorType.ToString() -eq 'Temperature' -and $null -ne $s.Value) {
                        if ($s.Name -match 'Tctl|CPU Package') {
                            $temp = $s.Value
                            break
                        }
                    }
                }
                if ($null -ne $temp) { break }
            }
            if ($null -ne $temp) { break }
        }

        # Fallback : n'importe quelle sonde CPU/Core
        if ($null -eq $temp) {
            foreach ($hw in $c.Hardware) {
                foreach ($s in $hw.Sensors) {
                    if ($s.SensorType.ToString() -eq 'Temperature' -and $null -ne $s.Value) {
                        if ($s.Name -match 'CPU|Core') {
                            $temp = $s.Value
                            break
                        }
                    }
                }
                if ($null -ne $temp) { break }
            }
        }

        if ($null -ne $temp) {
            [Console]::WriteLine($temp.ToString([System.Globalization.CultureInfo]::InvariantCulture))
        } else {
            [Console]::WriteLine('null')
        }
        [Console]::Out.Flush()

        Start-Sleep -Milliseconds 2000
    }
} finally {
    $c.Close()
}
