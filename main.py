"""
sysmon-widget — widget bureau système pour Windows 11

Prérequis :
  pip install psutil PyQt6 pynvml pythonnet

  Pour les températures CPU : télécharger LibreHardwareMonitor depuis
  https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/releases
  et copier LibreHardwareMonitorLib.dll dans le dossier hardware/

  Lancer en tant qu'administrateur pour activer la lecture des températures.
"""

import sys
from PyQt6.QtWidgets import QApplication
from PyQt6.QtCore import Qt

from widget import DesktopWidget
from monitor import MonitorThread


def main():
    QApplication.setHighDpiScaleFactorRoundingPolicy(
        Qt.HighDpiScaleFactorRoundingPolicy.PassThrough
    )
    app = QApplication(sys.argv)
    app.setQuitOnLastWindowClosed(True)

    widget = DesktopWidget()
    widget.show()

    monitor = MonitorThread(interval=2.0)
    monitor.data_updated.connect(widget.update_display)
    monitor.start()

    ret = app.exec()
    monitor.stop()
    sys.exit(ret)


if __name__ == '__main__':
    main()
