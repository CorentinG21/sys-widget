# install_startup.ps1
# Lancer en tant qu'Administrateur — enregistre SysmonWidget.exe comme tâche de démarrage.
# Placer ce script dans le même dossier que SysmonWidget.exe avant de l'exécuter.

$exePath = Join-Path $PSScriptRoot "SysmonWidget.exe"
$user    = "$env:USERDOMAIN\$env:USERNAME"

if (-not (Test-Path $exePath)) {
    Write-Host "ERREUR : SysmonWidget.exe introuvable dans $PSScriptRoot" -ForegroundColor Red
    Write-Host "Placez install_startup.ps1 a cote de SysmonWidget.exe et relancez." -ForegroundColor Red
    pause
    exit 1
}

Unregister-ScheduledTask -TaskName "SysmonWidget" -Confirm:$false -ErrorAction SilentlyContinue

$action    = New-ScheduledTaskAction -Execute $exePath -WorkingDirectory $PSScriptRoot
$trigger   = New-ScheduledTaskTrigger -AtLogOn -User $user
$settings  = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit (New-TimeSpan -Seconds 0)
$principal = New-ScheduledTaskPrincipal -UserId $user -LogonType Interactive -RunLevel Highest

Register-ScheduledTask -TaskName "SysmonWidget" -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Force

Write-Host "SysmonWidget ajoute au demarrage pour $env:USERNAME !" -ForegroundColor Green
pause
