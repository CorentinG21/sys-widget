# sysmon-widget

Widget de monitoring système pour Windows 11 — overlay frameless semi-transparent qui reste en bas du bureau.

## Stack

- **Python 3.14+** — pas de compilation, juste `pythonw.exe main.py`
- **PyQt6** — GUI frameless, signal/slot thread-safe
- **psutil** — CPU %, RAM, disques, réseau
- **pynvml** — GPU NVIDIA (graceful degradation si absent)
- **winreg** (stdlib) — réservé pour usage futur

## Architecture

```
main.py              → crée QApplication + MonitorThread + DesktopWidget, lance le tout
monitor.py           → QThread : collecte toutes les 2s, émet data_updated(dict)
collectors.py        → fonctions stateless de collecte (CPU, GPU, RAM, disques, réseau)
widget.py            → DesktopWidget : reçoit le signal, met à jour les QLabel
config.json          → position x/y persistée au drop du drag
```

## Dépendances

| Dépendance | Utilité | Note |
|---|---|---|
| **LibreHardwareMonitorLib.dll** | Températures CPU (AMD Tctl/Tdie, Intel CPU Package) | DLL dans `hardware/`, chargée par `hardware/read_temp.ps1` via PowerShell subprocess — pas de pythonnet, pas de LHM GUI (tâche planifiée LHM supprimée le 2026-05-07) |
| **pynvml** | GPU NVIDIA | Ignoré silencieusement si absent |

> `requirements.txt` liste les dépendances pip : `psutil>=5.9`, `PyQt6>=6.6`, `pynvml>=11.5`. Pythonnet n'est pas requis — la DLL est chargée via `subprocess` PowerShell natif.

## Lancer le widget

```powershell
# Interactif (avec console)
python main.py

# Silencieux (sans fenêtre console)
pythonw.exe main.py
```

> Admin requis (`app.manifest` → `requireAdministrator`). Le widget démarre automatiquement via Task Scheduler — `install_startup.ps1` crée la tâche `SysmonWidget` au logon pour `PC-DE-COCO\coren` avec niveau Highest.

## Données affichées

| Ligne | Source | Intervalle |
|---|---|---|
| CPU % + °C | psutil + LibreHardwareMonitorLib.dll (via PS subprocess) | 2 s |
| GPU % + °C + VRAM | pynvml | 2 s |
| RAM % + Go | psutil | 2 s |
| Disques % + Go | psutil (caché 10 s) | 10 s |
| Réseau ↑↓ MB/s | psutil delta | 2 s |

## Fonctionnalités du widget

- **Drag & drop** : clic gauche + glisser pour repositionner, position sauvegardée dans `config.json` au mouseRelease
- **Menu contextuel** (clic droit) : `Redémarrer` (relaunch `main.py` via subprocess) et `Quitter`
- **Taille dynamique** : `adjustSize()` appelé à chaque update — le widget s'adapte au nombre de disques détectés

## Températures CPU (`read_temp.ps1`)

Le script PS tourne en boucle persistante (processus fils de `collectors.py`), envoie une valeur par ligne sur stdout toutes les 2 s.

Priorité des sondes :
1. `Tctl` ou `CPU Package` (AMD / Intel principal)
2. Fallback : n'importe quel capteur dont le nom contient `CPU` ou `Core`
3. Si aucune sonde : émet `null` (affiché comme absent dans le widget)

## Modifier le polling

`monitor.py` ligne 9 : `MonitorThread(interval=2.0)` — changer `2.0` pour ajuster.

## Couleur des barres (CPU / GPU / RAM / Disques)

| Seuil | Couleur |
|---|---|
| < 70 % | Vert `#06d6a0` |
| 70–89 % | Jaune `#ffd166` |
| ≥ 90 % | Rouge `#ff6b6b` |

## Couleur réseau (seuils MB/s, pas %)

| Seuil | Upload | Download |
|---|---|---|
| < 1 MB/s | Vert `#06d6a0` | Cyan `#74d7f7` |
| 1–9 MB/s | Jaune `#ffd166` | Jaune `#ffd166` |
| ≥ 10 MB/s | Rouge `#ff6b6b` | Rouge `#ff6b6b` |
