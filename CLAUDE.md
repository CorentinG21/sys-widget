# sysmon-widget

Widget de monitoring système pour Windows 11 — overlay frameless semi-transparent qui reste en bas du bureau.

## Stack

- **Tauri v2** — shell Rust, fenêtre frameless transparente, toujours en bas (`setAlwaysOnBottom`)
- **SvelteKit + Svelte 5** — frontend (`$state`, `$derived`, runes)
- **TypeScript** — typage strict côté frontend
- **Rust** — backend : collecte métriques, subprocess LHM, startup schtask, updater
- **sysinfo** (Rust crate) — CPU %, RAM, disques, réseau
- **LibreHardwareMonitorLib.dll** — températures CPU/GPU via PowerShell subprocess (`hardware/read_temp.ps1`)

## Architecture

```
src/
  routes/+page.svelte       → layout principal, drag, ancrage, ouverture settings
  lib/
    components/
      MetricRow.svelte       → ligne CPU / GPU / RAM avec barre + sparkline
      DiskRow.svelte         → ligne disque
      NetworkRow.svelte      → ligne réseau ↑↓
      Sparkline.svelte       → mini graphe historique CPU/GPU
      SettingsPanel.svelte   → panel inline (couleur, transparence, ancrage, lock)
      ContextMenu.svelte     → menu clic droit (settings, update, startup, restart, quit)
      UpdateBanner.svelte    → bannière mise à jour disponible
    stores/
      metrics.svelte.ts      → $state métriques, listen('metrics-updated')
      settings.svelte.ts     → $state settings, load/save via plugin-store
    utils/
      colors.ts              → seuils couleurs, formatBytes

src-tauri/
  src/
    main.rs                  → entrée Tauri
    lib.rs                   → commandes Tauri, boucle métriques 2s, updater
    monitor.rs               → Monitor : CPU rolling avg, RAM, disques (cache 10s), réseau delta
    lhm.rs                   → LhmProcess : subprocess PS persistant, températures CPU/GPU
    models.rs                → structs MetricsPayload, CpuMetrics, GpuMetrics, etc.
    startup.rs               → schtask register/unregister/toggle via schtasks.exe
  hardware/
    read_temp.ps1            → script PS boucle infinie, émet JSON par ligne sur stdout
    LibreHardwareMonitorLib.dll

release.ps1                  → script de release sémantique (voir ci-dessous)
```

## Lancer en dev

```powershell
npm run tauri dev
```

## Release

Le script `release.ps1` bumpe la version dans `tauri.conf.json` + `Cargo.toml`, commit, push et tag — ce qui déclenche GitHub Actions pour builder et publier le `.exe`.

```powershell
# Syntaxe
.\release.ps1 patch "fix: description"   # x.y.Z+1
.\release.ps1 minor "feat: description"  # x.Y+1.0
.\release.ps1 major "feat: description"  # X+1.0.0

# Exemples
.\release.ps1 patch "fix: anchor panel adapts side on right corners + multi-monitor bounds"
.\release.ps1 minor "feat: nouvelle fonctionnalité"
```

> **Ne jamais pusher ni tagger manuellement** — toujours passer par `release.ps1`.

Releases : https://github.com/CorentinG21/sys-widget/releases

## Données affichées

| Ligne | Source | Intervalle |
|---|---|---|
| CPU % + °C | sysinfo + LHM DLL (via PS subprocess) | 2 s |
| GPU % + °C + VRAM | LHM DLL (via PS subprocess) | 2 s |
| RAM % + Go | sysinfo | 2 s |
| Disques % + Go | sysinfo (caché 10 s) | 10 s |
| Réseau ↑↓ MB/s | sysinfo delta | 2 s |
| Top process CPU | sysinfo | 2 s |

## Fonctionnalités

- **Drag & drop** : clic gauche + glisser, position sauvegardée dans `config.json` (plugin-store, `%APPDATA%`)
- **Verrouillage** : option "Verrouiller la position" dans settings
- **Menu contextuel** (clic droit) : Settings, Rechercher mise à jour, Démarrer avec Windows, Redémarrer, Quitter
- **Panel settings** (inline, s'ouvre à gauche ou droite selon position) :
  - Couleur accent : Cyan Néon / Vert Matrix / Blanc Épuré / Couleur libre (hue slider)
  - Transparence : slider continu 20–98%
  - Afficher/masquer les détails
  - Verrouiller la position
  - Ancrage aux 4 coins (adapte automatiquement le côté du panel)
- **Updater** : check auto 30s après lancement + toutes les heures, install depuis le menu
- **Startup** : toggle tâche planifiée Windows (`SysmonWidget`) depuis le menu contextuel

## Températures (`read_temp.ps1` + `lhm.rs`)

Le script PS tourne en boucle persistante (processus fils géré par `LhmProcess`), émet un objet JSON par ligne sur stdout toutes les 2 s. `lhm.rs` lit le dernier JSON valide reçu.

Priorité des sondes CPU :
1. `Tctl` ou `CPU Package` (AMD / Intel principal)
2. Fallback : n'importe quel capteur contenant `CPU` ou `Core`
3. Si aucune sonde : `null`

## Positionnement multi-écran

`openSettings()` et `anchorTo()` utilisent `currentMonitor()` de l'API Tauri v2 (bounds physiques réelles du moniteur courant) — contrairement à `window.screen` qui renvoie toujours le moniteur principal sous WebView2.

Ancrage droite (↗ / ↘) : le panel settings est automatiquement placé à **gauche** du widget.
Ancrage gauche (↖ / ↙) : le panel settings est à **droite**.

## Couleur des barres (CPU / GPU / RAM / Disques)

| Seuil | Couleur |
|---|---|
| < 70 % | Vert `#06d6a0` |
| 70–89 % | Jaune `#ffd166` |
| ≥ 90 % | Rouge `#ff6b6b` |

## Couleur réseau (seuils MB/s)

| Seuil | Upload | Download |
|---|---|---|
| < 1 MB/s | Vert `#06d6a0` | Cyan `#74d7f7` |
| 1–9 MB/s | Jaune `#ffd166` | Jaune `#ffd166` |
| ≥ 10 MB/s | Rouge `#ff6b6b` | Rouge `#ff6b6b` |

## Modifier le polling

`src-tauri/src/lib.rs` : `Duration::from_secs(2)` dans la boucle métriques.
