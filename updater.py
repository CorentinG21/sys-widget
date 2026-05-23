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

    # Bat attend que le process courant soit mort avant de copier le nouvel exe.
    pid = os.getpid()
    bat = os.path.join(tempfile.gettempdir(), 'sysmon_update.bat')
    with open(bat, 'w') as f:
        f.write('@echo off\n')
        f.write(':waitloop\n')
        f.write(f'tasklist /FI "PID eq {pid}" 2>NUL | find /I "{pid}" >NUL\n')
        f.write('if "%ERRORLEVEL%"=="0" (\n')
        f.write('    timeout /t 1 /nobreak >nul\n')
        f.write('    goto waitloop\n')
        f.write(')\n')
        f.write(f'copy /y "{tmp_exe}" "{current_exe}"\n')
        f.write(f'start "" "{current_exe}"\n')
        f.write('del "%~f0"\n')

    subprocess.Popen(
        ['cmd', '/c', bat],
        creationflags=subprocess.CREATE_NO_WINDOW | subprocess.DETACHED_PROCESS,
        close_fds=True,
    )
    return True
