import json
import os
import subprocess
import sys
import threading

from PyQt6.QtCore import Qt, QPoint, pyqtSignal, pyqtSlot
from PyQt6.QtGui import QPainter, QColor, QFont, QAction, QCursor
from PyQt6.QtWidgets import QApplication, QWidget, QVBoxLayout, QLabel, QMenu, QSizePolicy

import startup
import updater
from version import VERSION

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


class _ClickableLabel(QLabel):
    clicked = pyqtSignal()

    def mousePressEvent(self, event):
        if event.button() == Qt.MouseButton.LeftButton:
            self.clicked.emit()
        # Don't call super — prevent drag on this label

    def enterEvent(self, event):
        self.setCursor(QCursor(Qt.CursorShape.PointingHandCursor))
        super().enterEvent(event)

    def leaveEvent(self, event):
        self.setCursor(QCursor(Qt.CursorShape.ArrowCursor))
        super().leaveEvent(event)


class DesktopWidget(QWidget):
    _update_done = pyqtSignal(bool)      # True = success, False = failure
    _manual_update_signal = pyqtSignal(str, str)  # (version, url) — check manuel

    def __init__(self):
        super().__init__()
        self._drag_pos: QPoint | None = None
        self._update_url: str | None = None
        self._update_action: QAction | None = None
        self._init_ui()
        self._load_position()
        self._update_done.connect(self._on_update_done)
        self._manual_update_signal.connect(self.show_update)

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

        for key in ['cpu', 'gpu', 'ram']:
            lbl = _make_label(font)
            layout.addWidget(lbl)
            self._rows[key] = lbl

        sep_disk = QLabel('─' * 32)
        sep_disk.setFont(font)
        sep_disk.setStyleSheet('color: #383838;')
        layout.addWidget(sep_disk)

        self._disk_container = QWidget()
        self._disk_container.setSizePolicy(QSizePolicy.Policy.Preferred, QSizePolicy.Policy.Maximum)
        self._disk_layout = QVBoxLayout(self._disk_container)
        self._disk_layout.setContentsMargins(0, 0, 0, 0)
        self._disk_layout.setSpacing(3)
        layout.addWidget(self._disk_container)
        self._disk_labels: list[QLabel] = []

        sep_net = QLabel('─' * 32)
        sep_net.setFont(font)
        sep_net.setStyleSheet('color: #383838;')
        layout.addWidget(sep_net)

        net_lbl = _make_label(font)
        layout.addWidget(net_lbl)
        self._rows['net'] = net_lbl

        # Update notification bar (hidden until an update is detected)
        self._sep_update = QLabel('─' * 32)
        self._sep_update.setFont(font)
        self._sep_update.setStyleSheet('color: #383838;')
        self._sep_update.setVisible(False)
        layout.addWidget(self._sep_update)

        self._update_bar = _ClickableLabel()
        self._update_bar.setFont(font)
        self._update_bar.setStyleSheet('color: #ffd166;')
        self._update_bar.setVisible(False)
        self._update_bar.clicked.connect(self._start_update)
        layout.addWidget(self._update_bar)

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

        # GPU — masqué si aucun hardware GPU détecté
        if gpu:
            g_pct = gpu.get('usage')
            if g_pct is not None:
                bar_str = f'{_bar(g_pct)} {g_pct:>3.0f}%'
                gpu_color = _color_for(g_pct)
            else:
                bar_str = '—'
                gpu_color = '#606060'
            temp_str_g = f'  {gpu["temp"]:.0f}°C' if gpu.get('temp') is not None else ''
            vram_used = gpu.get('vram_used_gb')
            vram_total = gpu.get('vram_total_gb')
            vram_str = f'  {vram_used:.1f}/{vram_total:.0f}G' if vram_used is not None and vram_total is not None else ''
            self._rows['gpu'].setText(f'{"GPU":<6}{bar_str}{temp_str_g}{vram_str}')
            self._rows['gpu'].setStyleSheet(f'color: {gpu_color};')
            self._rows['gpu'].setVisible(True)
        else:
            self._rows['gpu'].setVisible(False)

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

    # ── Auto-update ──────────────────────────────────────────────────────────

    @pyqtSlot(str, str)
    def show_update(self, version: str, url: str):
        self._update_url = url
        self._update_bar.setText(f'Nouvelle version v{version} — Cliquer pour mettre a jour')
        self._sep_update.setVisible(True)
        self._update_bar.setVisible(True)
        if self._update_action:
            self._update_action.setText(f'Mettre a jour (v{version})')
            self._update_action.setVisible(True)
        self.adjustSize()

    def _start_update(self):
        if not self._update_url:
            return
        self._update_bar.setText('Telechargement en cours...')
        self._update_bar.setEnabled(False)
        if self._update_action:
            self._update_action.setEnabled(False)
        url = self._update_url
        threading.Thread(target=self._run_update, args=(url,), daemon=True).start()

    def _run_update(self, url: str):
        success = updater.download_and_apply(url)
        self._update_done.emit(success)

    @pyqtSlot(bool)
    def _on_update_done(self, success: bool):
        if success:
            QApplication.quit()
        else:
            self._update_bar.setText('Echec de la mise a jour')
            self._update_bar.setEnabled(True)
            if self._update_action:
                self._update_action.setEnabled(True)

    def _check_update_manual(self):
        """Déclenche un check de mise à jour dans un thread daemon (menu clic droit)."""
        def _check():
            result = updater.check_for_update(VERSION)
            if result:
                version, url = result
                self._manual_update_signal.emit(version, url)
        threading.Thread(target=_check, daemon=True).start()

    def _toggle_startup(self):
        """Active ou désactive le démarrage automatique avec Windows."""
        if startup.is_registered():
            startup.unregister()
        else:
            startup.register()

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

        version_action = QAction(f'SysmonWidget v{VERSION}', self)
        version_action.setEnabled(False)
        menu.addAction(version_action)
        menu.addSeparator()

        self._update_action = QAction('Mettre a jour', self)
        self._update_action.setVisible(self._update_url is not None)
        self._update_action.triggered.connect(self._start_update)
        menu.addAction(self._update_action)

        check_action = QAction('Rechercher une mise a jour', self)
        check_action.triggered.connect(self._check_update_manual)
        menu.addAction(check_action)
        menu.addSeparator()

        if getattr(sys, 'frozen', False):
            registered = startup.is_registered()
            label = '✓ Demarrer avec Windows' if registered else 'Demarrer avec Windows'
            startup_action = QAction(label, self)
            startup_action.triggered.connect(self._toggle_startup)
            menu.addAction(startup_action)
            menu.addSeparator()

        restart_action = QAction('Redemarrer', self)
        restart_action.triggered.connect(self._restart)
        menu.addAction(restart_action)
        menu.addSeparator()
        quit_action = QAction('Quitter', self)
        quit_action.triggered.connect(QApplication.quit)
        menu.addAction(quit_action)
        menu.exec(self.mapToGlobal(pos))

    def _restart(self):
        import collectors
        import tempfile
        from PyQt6.QtCore import QTimer
        collectors.cleanup()  # libère le verrou sur LibreHardwareMonitorLib.dll

        pid = os.getpid()

        if getattr(sys, 'frozen', False):
            exe = sys.executable
            direct_cmd = [exe]
        else:
            main_py = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'main.py')
            exe = None
            direct_cmd = [sys.executable, main_py]

        # ── Tentative 1 : intermédiaire PS ────────────────────────────────────
        # Attend la mort du vieux PID, supprime explicitement son _MEI, puis
        # relance l'exe. La suppression explicite est CRITIQUE : PyInstaller
        # utilise le même répertoire _MEI (basé sur le CRC de l'exe) pour toutes
        # les instances. Si la vieille instance ne peut pas le supprimer
        # entièrement (DLL encore chargées pendant atexit), le nouvel exe trouve
        # un _MEI partiel → "Failed to load python312.dll". En le supprimant
        # depuis le PS (après que tous les handles sont libérés par la mort du
        # process), on garantit une extraction fraîche et propre.
        mei_path = getattr(sys, '_MEIPASS', None)  # None si pas frozen
        ps_launched = False
        try:
            ps1 = os.path.join(tempfile.gettempdir(), 'sysmon_restart.ps1')
            if getattr(sys, 'frozen', False):
                mei_cleanup = (
                    f'Remove-Item -Path "{mei_path}" -Recurse -Force '
                    f'-ErrorAction SilentlyContinue\n'
                ) if mei_path else ''
                launch_lines = (
                    mei_cleanup +
                    f'Unblock-File -Path "{exe}" -ErrorAction SilentlyContinue\n'
                    f'Start-Process -FilePath "{exe}"\n'
                )
            else:
                launch_lines = (
                    f'Start-Process -FilePath "{sys.executable}" '
                    f'-ArgumentList "{main_py}"\n'
                )
            ps_content = (
                f'$p = {pid}\n'
                f'while (Get-Process -Id $p -ErrorAction SilentlyContinue)'
                f' {{ Start-Sleep -Milliseconds 200 }}\n'
                # 3s supplémentaires : garantit que le process est mort et que
                # Windows a libéré tous les handles sur les fichiers de _MEI,
                # même si Get-Process ne peut pas voir un process elevated.
                f'Start-Sleep -Seconds 3\n'
                + launch_lines +
                f'Remove-Item -Path "{ps1}" -Force -ErrorAction SilentlyContinue\n'
            )
            with open(ps1, 'w', encoding='utf-8') as f:
                f.write(ps_content)
            subprocess.Popen(
                ['powershell', '-NonInteractive', '-NoProfile',
                 '-ExecutionPolicy', 'Bypass', '-File', ps1],
                creationflags=(subprocess.CREATE_NO_WINDOW |
                               subprocess.CREATE_BREAKAWAY_FROM_JOB),
                close_fds=True,
            )
            ps_launched = True
        except Exception:
            pass

        # ── Fallback : relance directe si PS n'a pas pu demarrer ─────────────
        # Meme mecanisme que l'ancienne version — fonctionne, meme si le timing
        # peut provoquer un avertissement PyInstaller au premier lancement.
        if not ps_launched:
            try:
                subprocess.Popen(
                    direct_cmd,
                    creationflags=(subprocess.CREATE_NO_WINDOW |
                                   subprocess.DETACHED_PROCESS |
                                   subprocess.CREATE_BREAKAWAY_FROM_JOB),
                    close_fds=True,
                )
            except Exception:
                pass

        # QTimer.singleShot garantit que le quit est traité APRES que le slot
        # et menu.exec() se soient termines (retour dans la main event loop).
        QTimer.singleShot(200, QApplication.quit)

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
