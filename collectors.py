import json
import os
import sys
import time
import threading
import subprocess
import psutil

# ── CPU / GPU via PowerShell + LibreHardwareMonitorLib.dll ───────────────────
# Un processus PS persistant lit la DLL .NET et envoie du JSON sur stdout.
# Supporte CPU (AMD Tctl/Tdie, Intel Package) et GPU (NVIDIA/AMD/Intel).

def _get_base():
    if getattr(sys, 'frozen', False):
        return sys._MEIPASS
    return os.path.dirname(os.path.abspath(__file__))

_PS_SCRIPT = os.path.join(_get_base(), 'hardware', 'read_temp.ps1')

_lhm_proc: subprocess.Popen | None = None
_lhm_temp: float | None = None
_lhm_gpu: dict | None = None
_lhm_failed = False
_lhm_lock = threading.Lock()


def _lhm_reader():
    global _lhm_temp, _lhm_gpu, _lhm_failed, _lhm_proc
    while _lhm_proc is not None:
        try:
            line = _lhm_proc.stdout.readline()
            if not line:
                break
            val = line.strip()
            if not val:
                continue
            try:
                data = json.loads(val)
            except (json.JSONDecodeError, ValueError):
                # Backward compat: legacy format was a single float
                with _lhm_lock:
                    _lhm_temp = float(val) if val != 'null' else None
                continue
            with _lhm_lock:
                cpu_t = data.get('cpu_temp')
                _lhm_temp = float(cpu_t) if cpu_t is not None else None
                gpu_d = data.get('gpu')
                if gpu_d:
                    vram_used = gpu_d.get('vram_used_mb')
                    vram_total = gpu_d.get('vram_total_mb')
                    _lhm_gpu = {
                        'usage': float(gpu_d['usage']) if gpu_d.get('usage') is not None else 0.0,
                        'temp': float(gpu_d['temp']) if gpu_d.get('temp') is not None else None,
                        'vram_used_gb': vram_used / 1024 if vram_used is not None else None,
                        'vram_total_gb': vram_total / 1024 if vram_total is not None else None,
                    }
                else:
                    _lhm_gpu = None
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


def get_gpu() -> dict | None:
    _init_lhm()
    with _lhm_lock:
        if _lhm_gpu is not None:
            return _lhm_gpu
    return _get_gpu_pynvml()


# ── NVIDIA fallback via pynvml (si LHM ne détecte pas le GPU) ────────────────

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


def _get_gpu_pynvml() -> dict | None:
    global _nvml_handle
    if _nvml_handle is None and not _init_nvml():
        return None
    try:
        import pynvml
        util = pynvml.nvmlDeviceGetUtilizationRates(_nvml_handle)
        temp = pynvml.nvmlDeviceGetTemperature(_nvml_handle, pynvml.NVML_TEMPERATURE_GPU)
        mem = pynvml.nvmlDeviceGetMemoryInfo(_nvml_handle)
        return {
            'usage': float(util.gpu),
            'temp': float(temp),
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
    global _lhm_proc, _nvml_handle
    if _lhm_proc is not None:
        try:
            _lhm_proc.terminate()
            _lhm_proc.wait(timeout=3)  # attend la mort réelle pour libérer le DLL
            _lhm_proc = None
        except Exception:
            pass
    if _nvml_handle is not None:
        try:
            import pynvml
            pynvml.nvmlShutdown()
        except Exception:
            pass
