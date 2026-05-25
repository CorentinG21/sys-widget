import os
import subprocess
import sys

TASK_NAME = 'SysmonWidget'
_CF = subprocess.CREATE_NO_WINDOW


def is_registered() -> bool:
    """Retourne True si la tâche de démarrage SysmonWidget existe dans Task Scheduler."""
    r = subprocess.run(
        ['schtasks', '/Query', '/TN', TASK_NAME, '/FO', 'LIST'],
        capture_output=True,
        creationflags=_CF,
    )
    return r.returncode == 0


def register() -> bool:
    """
    Crée (ou remplace) la tâche planifiée qui lance le widget au logon.
    Ne fonctionne qu'en mode frozen (exe compilé). Requiert les droits admin.
    Reproduit exactement ce que fait install_startup.ps1.
    """
    if not getattr(sys, 'frozen', False):
        return False

    exe = sys.executable
    workdir = os.path.dirname(exe)
    user = os.environ.get('USERDOMAIN', '.') + '\\' + os.environ.get('USERNAME', '')

    ps = (
        f'$a = New-ScheduledTaskAction -Execute "{exe}" -WorkingDirectory "{workdir}"\n'
        f'$t = New-ScheduledTaskTrigger -AtLogOn -User "{user}"\n'
        f'$s = New-ScheduledTaskSettingsSet '
        f'-AllowStartIfOnBatteries -DontStopIfGoingOnBatteries '
        f'-ExecutionTimeLimit (New-TimeSpan -Seconds 0)\n'
        f'$p = New-ScheduledTaskPrincipal -UserId "{user}" '
        f'-LogonType Interactive -RunLevel Highest\n'
        f'Register-ScheduledTask -TaskName "{TASK_NAME}" '
        f'-Action $a -Trigger $t -Settings $s -Principal $p -Force\n'
    )
    r = subprocess.run(
        ['powershell', '-NonInteractive', '-NoProfile',
         '-ExecutionPolicy', 'Bypass', '-Command', ps],
        capture_output=True,
        creationflags=_CF,
    )
    return r.returncode == 0


def unregister() -> bool:
    """Supprime la tâche de démarrage SysmonWidget."""
    r = subprocess.run(
        ['powershell', '-NonInteractive', '-NoProfile',
         '-ExecutionPolicy', 'Bypass', '-Command',
         f'Unregister-ScheduledTask -TaskName "{TASK_NAME}" '
         f'-Confirm:$false -ErrorAction SilentlyContinue'],
        capture_output=True,
        creationflags=_CF,
    )
    return r.returncode == 0
