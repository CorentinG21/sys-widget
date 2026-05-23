import json
import os
import subprocess
import sys
import tempfile
import urllib.request

GITHUB_API = "https://api.github.com/repos/CorentinG21/sys-widget/releases/latest"
_HEADERS = {'User-Agent': 'sysmon-widget'}


def _ver(v: str) -> tuple:
    try:
        return tuple(int(x) for x in v.lstrip('v').split('.'))
    except Exception:
        return (0,)


def check_for_update(current_version: str) -> tuple[str, str] | None:
    """Returns (latest_version, download_url) if a newer release exists, else None."""
    try:
        req = urllib.request.Request(GITHUB_API, headers=_HEADERS)
        with urllib.request.urlopen(req, timeout=10) as resp:
            data = json.loads(resp.read())
        tag = data.get('tag_name', '').lstrip('v')
        if not tag or _ver(tag) <= _ver(current_version):
            return None
        for asset in data.get('assets', []):
            if asset['name'].lower().endswith('.exe'):
                return tag, asset['browser_download_url']
    except Exception:
        pass
    return None


def download_and_apply(download_url: str, on_progress=None) -> bool:
    """
    Download the new exe and schedule a self-replace via a bat script after exit.
    Returns True if the download succeeded and the bat was launched.
    Only works when running as a frozen exe (sys.frozen).
    """
    if not getattr(sys, 'frozen', False):
        return False

    current_exe = sys.executable
    tmp_exe = os.path.join(tempfile.gettempdir(), 'SysmonWidget_new.exe')

    try:
        req = urllib.request.Request(download_url, headers=_HEADERS)
        with urllib.request.urlopen(req, timeout=120) as resp, open(tmp_exe, 'wb') as f:
            total = int(resp.headers.get('Content-Length', 0))
            downloaded = 0
            while True:
                chunk = resp.read(65536)
                if not chunk:
                    break
                f.write(chunk)
                downloaded += len(chunk)
                if on_progress and total:
                    on_progress(downloaded / total)
    except Exception:
        return False

    # Script PowerShell : attend la mort du process, copie, débloque, relance.
    # Plus fiable qu'un bat pour les exe avec manifest requireAdministrator.
    pid = os.getpid()
    ps1 = os.path.join(tempfile.gettempdir(), 'sysmon_update.ps1')
    ps_content = (
        f'$p = {pid}\n'
        f'while (Get-Process -Id $p -ErrorAction SilentlyContinue) {{ Start-Sleep -Seconds 1 }}\n'
        f'Start-Sleep -Seconds 1\n'
        f'Unblock-File -Path "{tmp_exe}" -ErrorAction SilentlyContinue\n'
        f'Copy-Item -Path "{tmp_exe}" -Destination "{current_exe}" -Force\n'
        f'Unblock-File -Path "{current_exe}" -ErrorAction SilentlyContinue\n'
        f'Start-Process -FilePath "{current_exe}"\n'
        f'Remove-Item -Path "{ps1}" -Force -ErrorAction SilentlyContinue\n'
    )
    with open(ps1, 'w', encoding='utf-8') as f:
        f.write(ps_content)

    subprocess.Popen(
        ['powershell', '-NonInteractive', '-NoProfile',
         '-ExecutionPolicy', 'Bypass', '-File', ps1],
        creationflags=(subprocess.CREATE_NO_WINDOW |
                       subprocess.DETACHED_PROCESS |
                       subprocess.CREATE_BREAKAWAY_FROM_JOB),
        close_fds=True,
    )
    return True
