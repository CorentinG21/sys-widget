import threading
import time

from PyQt6.QtCore import QThread, pyqtSignal

import collectors
import updater
from version import VERSION


class MonitorThread(QThread):
    data_updated = pyqtSignal(dict)
    update_available = pyqtSignal(str, str)  # (version, download_url)

    def __init__(self, interval: float = 2.0):
        super().__init__()
        self.interval = interval
        self._running = True
        self._stop_event = threading.Event()

    def run(self):
        # Prime psutil cpu_percent (first call always returns 0.0)
        collectors.get_cpu_usage()
        time.sleep(0.5)

        threading.Thread(target=self._update_check_loop, daemon=True).start()

        while self._running:
            data = {
                'cpu_usage': collectors.get_cpu_usage(),
                'cpu_temp': collectors.get_cpu_temp(),
                'gpu': collectors.get_gpu(),
                'ram': collectors.get_ram(),
                'disks': collectors.get_disks(),
                'network': collectors.get_network(),
            }
            self.data_updated.emit(data)
            time.sleep(self.interval)

    def _update_check_loop(self):
        # Wait 30s after startup, then check every 6 hours
        if self._stop_event.wait(30):
            return
        while self._running:
            result = updater.check_for_update(VERSION)
            if result:
                version, url = result
                self.update_available.emit(version, url)
                return
            if self._stop_event.wait(6 * 3600):
                return

    def stop(self):
        self._running = False
        self._stop_event.set()
        self.wait()
        collectors.cleanup()
