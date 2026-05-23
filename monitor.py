import time
from PyQt6.QtCore import QThread, pyqtSignal
import collectors


class MonitorThread(QThread):
    data_updated = pyqtSignal(dict)

    def __init__(self, interval: float = 2.0):
        super().__init__()
        self.interval = interval
        self._running = True

    def run(self):
        # Prime psutil cpu_percent (first call always returns 0.0)
        collectors.get_cpu_usage()
        time.sleep(0.5)

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

    def stop(self):
        self._running = False
        self.wait()
        collectors.cleanup()
