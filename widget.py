import json
import os
import subprocess
import sys

from PyQt6.QtCore import Qt, QPoint, pyqtSlot
from PyQt6.QtGui import QPainter, QColor, QFont, QAction
from PyQt6.QtWidgets import QApplication, QWidget, QVBoxLayout, QLabel, QMenu, QSizePolicy

_appdata = os.environ.get('APPDATA', os.path.expanduser('~'))
_config_dir = os.path.join(_appdata, 'sysmon-widget')
os.makedirs(_config_dir, exist_ok=True)
CONFIG_FILE = os.path.join(_config_dir, 'config.json')

_BAR_WIDTH = 10


def _bar(percent: float) -> str:
    filled = round(max(0, min(100, percent)) / 100 * _BAR_WIDTH)
    return '█' * filled + '░' * (_BAR_WIDTH - filled)


def _fmt_rate(mb: float) -> str:
    if mb >= 1:
        return f'{mb:.2f} MB/s'
    kb = mb * 1024
    if kb >= 1:
        return f'{kb:.1f} KB/s'
    return f'{kb * 1024:.0f} B/s'


def _color_for(percent: float) -> str:
    if percent >= 90:
        return '#ff6b6b'
    if percent >= 70:
        return '#ffd166'
    return '#06d6a0'


def _color_for_net(mb: float, is_up: bool) -> str:
    if mb >= 10:
        return '#ff6b6b'
    if mb >= 1:
        return '#ffd166'
    return '#06d6a0' if is_up else '#74d7f7'


def _make_label(font: QFont, color: str = '#c8c8c8') -> QLabel:
    lbl = QLabel('—')
    lbl.setFont(font)
    lbl.setStyleSheet(f'color: {color};')
    return lbl


class DesktopWidget(QWidget):
    def __init__(self):
        super().__init__()
        self._drag_pos: QPoint | None = None
        self._init_ui()
        self._load_position()

    def _init_ui(self):
        self.setWindowFlags(
            Qt.WindowType.FramelessWindowHint |
            Qt.WindowType.WindowStaysOnBottomHint |
            Qt.WindowType.Tool
        )
        self.setAttribute(Qt.WidgetAttribute.WA_TranslucentBackground)
        self.setContextMenuPolicy(Qt.ContextMenuPolicy.CustomContextMenu)
        self.customContextMenuRequested.connect(self._show_context_menu)

        font = QFont('Consolas', 9)
        self._font = font

        layout = QVBoxLayout(self)
        layout.setContentsMargins(16, 14, 16, 14)
        layout.setSpacing(3)

        self._rows: dict[str, QLabel] = {}

        # CPU / GPU / RAM — une ligne chacun
        for key in ['cpu', 'gpu', 'ram']:
            lbl = _make_label(font)
            layout.addWidget(lbl)
            self._rows[key] = lbl

        # Séparateur composants / disques
        sep_disk = QLabel('─' * 32)
        sep_disk.setFont(font)
        sep_disk.setStyleSheet('color: #383838;')
        layout.addWidget(sep_disk)

        # Disques dynamiques
        self._disk_container = QWidget()
        self._disk_container.setSizePolicy(QSizePolicy.Policy.Preferred, QSizePolicy.Policy.Maximum)
        self._disk_layout = QVBoxLayout(self._disk_container)
        self._disk_layout.setContentsMargins(0, 0, 0, 0)
        self._disk_layout.setSpacing(3)
        layout.addWidget(self._disk_container)
        self._disk_labels: list[QLabel] = []

        # Séparateur avant réseau
        sep = QLabel('─' * 32)
        sep.setFont(font)
        sep.setStyleSheet('color: #383838;')
        layout.addWidget(sep)

        # Réseau
        net_lbl = _make_label(font)
        layout.addWidget(net_lbl)
        self._rows['net'] = net_lbl

        self.adjustSize()

    def paintEvent(self, event):
        p = QPainter(self)
        p.setRenderHint(QPainter.RenderHint.Antialiasing)
        p.setBrush(QColor(10, 10, 10, 185))
        p.setPen(QColor(55, 55, 55, 220))
        p.drawRoundedRect(self.rect().adjusted(1, 1, -1, -1), 10, 10)

    @pyqtSlot(dict)
    def update_display(self, data: dict):
        cpu_pct = data.get('cpu_usage', 0.0)
        cpu_temp = data.get('cpu_temp')
        gpu = data.get('gpu')
        ram = data.get('ram', {})
        disks = data.get('disks', [])
        net = data.get('network', {})

        # CPU
        temp_str = f'  {cpu_temp:.0f}°C' if cpu_temp is not None else ''
        self._rows['cpu'].setText(f'{"CPU":<6}{_bar(cpu_pct)} {cpu_pct:>3.0f}%{temp_str}')
        self._rows['cpu'].setStyleSheet(f'color: {_color_for(cpu_pct)};')

        # GPU
        if gpu:
            g_pct = gpu['usage']
            vram = f"  {gpu['vram_used_gb']:.1f}/{gpu['vram_total_gb']:.0f}G"
            self._rows['gpu'].setText(f'{"GPU":<6}{_bar(g_pct)} {g_pct:>3}%  {gpu["temp"]}°C{vram}')
            self._rows['gpu'].setStyleSheet(f'color: {_color_for(g_pct)};')
        else:
            self._rows['gpu'].setText('GPU   N/A')
            self._rows['gpu'].setStyleSheet('color: #606060;')

        # RAM
        ram_pct = ram.get('percent', 0.0)
        used = ram.get('used_gb', 0)
        total = ram.get('total_gb', 0)
        self._rows['ram'].setText(f'{"RAM":<6}{_bar(ram_pct)} {ram_pct:>3.0f}%  {used:.1f}/{total:.0f}G')
        self._rows['ram'].setStyleSheet(f'color: {_color_for(ram_pct)};')

        # Disques — reconstruit si le nombre change
        if len(disks) != len(self._disk_labels):
            for lbl in self._disk_labels:
                lbl.deleteLater()
            self._disk_labels = []
            for _ in disks:
                lbl = QLabel()
                lbl.setFont(self._font)
                self._disk_layout.addWidget(lbl)
                self._disk_labels.append(lbl)

        for i, disk in enumerate(disks):
            pct = disk['percent']
            mount = disk['mountpoint'].rstrip('\\')
            used_d = disk['used_gb']
            total_d = disk['total_gb']
            self._disk_labels[i].setText(f'{mount:<6}{_bar(pct)} {pct:>3.0f}%  {used_d:.0f}/{total_d:.0f}G')
            self._disk_labels[i].setStyleSheet(f'color: {_color_for(pct)};')

        # Réseau
        up_mb = net.get('up_mb', 0.0)
        dn_mb = net.get('down_mb', 0.0)
        up_color = _color_for_net(up_mb, True)
        dn_color = _color_for_net(dn_mb, False)
        self._rows['net'].setText(
            f'<span style="color:{up_color};">↑&nbsp;{_fmt_rate(up_mb)}</span>'
            f'&nbsp;&nbsp;&nbsp;'
            f'<span style="color:{dn_color};">↓&nbsp;{_fmt_rate(dn_mb)}</span>'
        )

        self.adjustSize()

    # ── Drag ────────────────────────────────────────────────────────────────

    def mousePressEvent(self, event):
        if event.button() == Qt.MouseButton.LeftButton:
            self._drag_pos = event.globalPosition().toPoint() - self.frameGeometry().topLeft()

    def mouseMoveEvent(self, event):
        if self._drag_pos and event.buttons() == Qt.MouseButton.LeftButton:
            self.move(event.globalPosition().toPoint() - self._drag_pos)

    def mouseReleaseEvent(self, event):
        self._drag_pos = None
        self._save_position()

    # ── Context menu ─────────────────────────────────────────────────────────

    def _show_context_menu(self, pos):
        menu = QMenu(self)
        restart_action = QAction('Redémarrer', self)
        restart_action.triggered.connect(self._restart)
        menu.addAction(restart_action)
        menu.addSeparator()
        quit_action = QAction('Quitter', self)
        quit_action.triggered.connect(QApplication.quit)
        menu.addAction(quit_action)
        menu.exec(self.mapToGlobal(pos))

    def _restart(self):
        if getattr(sys, 'frozen', False):
            subprocess.Popen([sys.executable], creationflags=subprocess.CREATE_NO_WINDOW)
        else:
            script = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'main.py')
            subprocess.Popen([sys.executable, script], creationflags=subprocess.CREATE_NO_WINDOW)
        QApplication.quit()

    # ── Position persistence ─────────────────────────────────────────────────

    def _load_position(self):
        try:
            with open(CONFIG_FILE) as f:
                cfg = json.load(f)
            self.move(cfg.get('x', 50), cfg.get('y', 50))
        except Exception:
            self.move(50, 50)

    def _save_position(self):
        pos = self.pos()
        try:
            with open(CONFIG_FILE, 'w') as f:
                json.dump({'x': pos.x(), 'y': pos.y()}, f)
        except Exception:
            pass

    def closeEvent(self, event):
        self._save_position()
        super().closeEvent(event)
