import os
import sys
import time
import threading
import subprocess
import psutil

# ── CPU temperatures via PowerShell + LibreHardwareMonitorLib.dll ────────────
# Un processus PS persistent lit la DLL .NET et envoie la temp sur stdout.
# Python lit ces valeurs dans un thread daemon — pas de fenêtre LHM, pas de pythonnet.

def _get_base():
    if getattr(sys, 'frozen', False):
        return sys._MEIPASS
    return os.path.dirname(os.path.abspath(__file__))

_PS_SCRIPT = os.path.join(_get_base(), 'hardware', 'read_temp.ps1')

_lhm_proc: subprocess.Popen | None = None
_lhm_temp: float | None = None
_lhm_failed = False
_lhm_lock = threading.Lock()


def _lhm_reader():
    global _lhm_temp, _lhm_failed, _lhm_proc
    while _lhm_proc is not None:
        try:
            line = _lhm_proc.stdout.readline()
            if not line:
                break
            val = line.strip()
            if val and val != 'null':
                with _lhm_lock:
                    _lhm_temp = float(val)
        except Exception:
            break
    _lhm_failed = True


def _init_lhm() -> bool:
    global _lhm_proc, _lhm_failed
    if _lhm_failed:
        return False
    if _lhm_proc is not None:
        return True
    if not os.path.exists(_PS_SCRIPT):
        _lhm_failed = True
        return False
    try:
        _lhm_proc = subprocess.Popen(
            ['powershell', '-NonInteractive', '-NoProfile',
             '-ExecutionPolicy', 'Bypass', '-File', _PS_SCRIPT],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
        threading.Thread(target=_lhm_reader, daemon=True).start()
        return True
    except Exception:
        _lhm_failed = True
        return False


def get_cpu_usage() -> float:
    return psutil.cpu_percent(interval=None)


def get_cpu_temp() -> float | None:
    _init_lhm()
    with _lhm_lock:
        return _lhm_temp

# ── NVIDIA GPU ───────────────────────────────────────────────────────────────

_nvml_handle = None
_nvml_failed = False

def _init_nvml():
    global _nvml_handle, _nvml_failed
    if _nvml_failed:
        return False
    try:
        import pynvml
        pynvml.nvmlInit()
        _nvml_handle = pynvml.nvmlDeviceGetHandleByIndex(0)
        return True
    except Exception:
        _nvml_failed = True
        return False

def get_gpu() -> dict | None:
    global _nvml_handle
    if _nvml_handle is None:
        if not _init_nvml():
            return None
    try:
        import pynvml
        util = pynvml.nvmlDeviceGetUtilizationRates(_nvml_handle)
        temp = pynvml.nvmlDeviceGetTemperature(_nvml_handle, pynvml.NVML_TEMPERATURE_GPU)
        mem = pynvml.nvmlDeviceGetMemoryInfo(_nvml_handle)
        return {
            'usage': util.gpu,
            'temp': temp,
            'vram_used_gb': mem.used / 1024 ** 3,
            'vram_total_gb': mem.total / 1024 ** 3,
        }
    except Exception:
        return None

# ── RAM ──────────────────────────────────────────────────────────────────────

def get_ram() -> dict:
    m = psutil.virtual_memory()
    return {
        'percent': m.percent,
        'used_gb': m.used / 1024 ** 3,
        'total_gb': m.total / 1024 ** 3,
    }

# ── Disks (cached 10s) ───────────────────────────────────────────────────────

_disk_cache: list | None = None
_disk_cache_ts: float = 0.0

def get_disks() -> list[dict]:
    global _disk_cache, _disk_cache_ts
    now = time.monotonic()
    if _disk_cache is None or now - _disk_cache_ts > 10:
        disks = []
        for part in psutil.disk_partitions(all=False):
            if 'cdrom' in part.opts or part.fstype == '':
                continue
            try:
                u = psutil.disk_usage(part.mountpoint)
                disks.append({
                    'mountpoint': part.mountpoint,
                    'percent': u.percent,
                    'used_gb': u.used / 1024 ** 3,
                    'total_gb': u.total / 1024 ** 3,
                })
            except Exception:
                continue
        _disk_cache = disks
        _disk_cache_ts = now
    return _disk_cache

# ── Network (delta between calls) ────────────────────────────────────────────

_prev_net = None
_prev_net_ts: float = 0.0

def get_network() -> dict:
    global _prev_net, _prev_net_ts
    now = time.monotonic()
    net = psutil.net_io_counters()
    if _prev_net is None:
        _prev_net = net
        _prev_net_ts = now
        return {'up_mb': 0.0, 'down_mb': 0.0}
    elapsed = now - _prev_net_ts
    up = (net.bytes_sent - _prev_net.bytes_sent) / elapsed / 1024 ** 2
    down = (net.bytes_recv - _prev_net.bytes_recv) / elapsed / 1024 ** 2
    _prev_net = net
    _prev_net_ts = now
    return {'up_mb': max(0.0, up), 'down_mb': max(0.0, down)}

# ── Cleanup ──────────────────────────────────────────────────────────────────

def cleanup():
    global _nvml_handle, _lhm_proc
    if _lhm_proc is not None:
        try:
            _lhm_proc.terminate()
            _lhm_proc = None
        except Exception:
            pass
    if _nvml_handle is not None:
        try:
            import pynvml
            pynvml.nvmlShutdown()
        except Exception:
            pass
