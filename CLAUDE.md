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
    shortcuts.ts             → raccourci global du click-through (constante unique)
    utils/
      colors.ts              → seuils couleurs, formatBytes, latencyColor
      colors.test.ts         → tests Vitest pour toutes les fonctions de colors.ts
      geometry.ts            → calculs purs de fenêtre : ancrage, moniteurs, position widget ↔ fenêtre, menu
      geometry.test.ts       → tests Vitest de geometry.ts

src-tauri/
  src/
    main.rs                  → entrée Tauri
    lib.rs                   → commandes Tauri, boucle métriques 2s, updater
    monitor.rs               → Monitor : CPU rolling avg, RAM, disques (cache 10s), réseau delta, latence TCP
    lhm.rs                   → LhmProcess : subprocess PS persistant, températures CPU/GPU
    models.rs                → structs MetricsPayload, CpuMetrics, GpuMetrics, etc.
    startup.rs               → schtask register/unregister/toggle via schtasks.exe, réparation des tâches obsolètes
    legacy.rs                → lance cleanup_legacy.ps1 au démarrage (build release uniquement)
  hardware/
    read_temp.ps1            → script PS en boucle, émet JSON par ligne sur stdout, s'arrête quand le widget (-ParentPid) disparaît
    LibreHardwareMonitorLib.dll + 11 DLL de dépendances (builds .NET Framework)
    THIRD-PARTY-NOTICES.txt  → versions, licences et sources des DLL
    cleanup_legacy.ps1       → supprime l'ancienne install par utilisateur (≤ 2.9.0) et réoriente ses raccourcis
  tests/
    read_temp.test.ps1       → tests du script (sélection des sondes sur faux matériel + comportement du processus)
    cleanup_legacy.test.ps1  → tests du nettoyage sur une fausse machine (dossier temporaire + clé HKCU de test)

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
| Réseau ↑↓ MB/s + latence ms | sysinfo delta + TCP connect 8.8.8.8:53 | 2 s / 5 s |
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

Priorité des sondes CPU (matériel de type `Cpu` uniquement, jamais le GPU) :
1. `Tctl`, `Tdie` ou `CPU Package` (AMD / Intel principal)
2. Fallback : n'importe quel capteur contenant `CPU` ou `Core` (hors « Distance to TjMax »)
3. Si aucune sonde : fallback WMI ThermalZone, sinon `null`

Une sonde CPU à 0 est ignorée : c'est ce que renvoie LHM sans droits admin ou sans le pilote PawnIO (cas de `tauri dev`).

> ⚠ **DLL : build .NET Framework obligatoire.** Le script tourne dans Windows PowerShell 5.1 (.NET Framework 4.x) : un build .NET 8/9/10 de LibreHardwareMonitorLib ne s'y charge pas, et le script retombe silencieusement sur les fallbacks (`nvidia-smi`, compteurs Windows, WMI). `hardware/` contient le build `net472` de LibreHardwareMonitorLib 0.9.6 et ses 11 dépendances ; versions, licences et procédure de mise à jour dans `hardware/THIRD-PARTY-NOTICES.txt`. Le test `process: loads LibreHardwareMonitor` de `read_temp.test.ps1` vérifie le chargement. Si la lib échoue malgré tout, la raison apparaît en `[lhm:stderr]` dans la console de `tauri dev`.

## Fenêtre et positionnement (`+page.svelte` + `utils/geometry.ts`)

- La fenêtre fait toujours exactement la taille de son contenu (`ResizeObserver` → `syncWindow()`), pour qu'aucune zone transparente ne capte les clics du bureau.
- Source de vérité : `widgetPos`, coin haut-gauche du **widget** à l'écran en pixels physiques. Le rectangle de la fenêtre en est dérivé ; c'est aussi `widgetPos` qui est sauvegardé (clé `position`), jamais la position de la fenêtre agrandie.
- Ancrage : la clé `anchor` mémorise le coin ; il est ré-appliqué à chaque changement de taille du widget et annulé par un déplacement manuel. Les calculs utilisent la zone de travail du moniteur (`workArea`, hors barre des tâches).
- Le panel settings passe à **gauche** s'il dépasse le bord droit ; le panel et la place du menu contextuel s'étendent **vers le haut** quand le widget est dans la moitié basse de l'écran.
- Au démarrage, une position sauvegardée sur un écran débranché est ramenée dans l'écran le plus proche. Tray → « Réinitialiser la position » ancre en haut à droite.

## Couleur des barres (CPU / GPU / RAM / Disques)

| Seuil | Couleur |
|---|---|
| < 70 % | Vert `#06d6a0` |
| 70–89 % | Jaune `#ffd166` |
| ≥ 90 % | Rouge `#ff6b6b` |

## Latence réseau (`monitor.rs` + `NetworkRow.svelte`)

`start_latency_poller()` démarre un thread dédié qui tente un TCP connect vers `8.8.8.8:53` toutes les 5 s (timeout 2 s). Le résultat est stocké dans un `Arc<Mutex<Option<u32>>>` et injecté dans `NetworkMetrics.latency_ms` à chaque tick.

Affiché dans `NetworkRow` sous forme `• Xms` (ou `• --` si offline).

| Seuil | Couleur |
|---|---|
| < 30 ms | Vert `--color-ok` |
| 30–100 ms | Jaune `--color-warn` |
| > 100 ms | Rouge `--color-danger` |
| null | Gris (offline / timeout) |

## Couleur réseau (seuils MB/s)

| Seuil | Upload | Download |
|---|---|---|
| < 1 MB/s | Vert `#06d6a0` | Cyan `#74d7f7` |
| 1–9 MB/s | Jaune `#ffd166` | Jaune `#ffd166` |
| ≥ 10 MB/s | Rouge `#ff6b6b` | Rouge `#ff6b6b` |

## Tests

### TypeScript (Vitest)
```powershell
npm run test        # run once
npm run test:watch  # watch mode
```
Tests dans `src/lib/utils/colors.test.ts` (`thresholdColor`, `netColors`, `formatRate`, `formatBytes`, `latencyColor`) et `src/lib/utils/geometry.test.ts` (ancrage, moniteurs, position widget ↔ fenêtre, placement du menu).

### Rust
```powershell
cargo test --manifest-path src-tauri/Cargo.toml
```
Tests dans `monitor.rs` (latence, sérialisation JSON, collect), `lhm.rs` (parsing JSON, arguments PowerShell) et `startup.rs` (tâche planifiée : script d'enregistrement, détection d'une tâche obsolète).

### Script des sondes (PowerShell)
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File src-tauri\tests\read_temp.test.ps1
```
Sélection des sondes CPU/GPU sur du faux matériel, et comportement du vrai script lancé comme processus (sortie JSON, chargement de LHM, arrêt quand le parent disparaît, chemin `\\?\`). Code de sortie = nombre d'échecs.

### Nettoyage de l'ancienne installation (PowerShell)
```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File src-tauri\tests\cleanup_legacy.test.ps1
```
Chaque test construit une fausse machine (dossier temporaire + `HKCU:\Software\SysmonWidgetTests`) et lance le vrai script dessus ; rien d'autre n'est touché.

## Migration per-user → per-machine (`cleanup_legacy.ps1`)

Jusqu'à 2.9.0 l'appli s'installait dans `AppData\Local` (par utilisateur), depuis 2.10.0 dans Program Files. La mise à jour NSIS n'enlève pas l'ancienne copie, et les raccourcis de l'utilisateur continuent de la lancer (en admin, depuis un dossier modifiable sans droits admin). À chaque démarrage d'un build release, le widget lance `cleanup_legacy.ps1`, qui :
- ne fait rien sans entrée de désinstallation HKCU, ou si l'exe ne tourne pas depuis l'install per-machine (`InstallLocation` HKLM) ;
- réoriente vers l'exe courant les `.lnk` (menu Démarrer, Bureau et sous-dossiers, Démarrage, barre des tâches) qui visent l'ancien exe ;
- supprime uniquement les fichiers connus de l'ancienne install, puis les dossiers vides, puis l'entrée HKCU et `HKCU\Software\<publisher>\SysmonWidget`.

Il tourne en admin mais lit des valeurs modifiables par l'utilisateur : il refuse tout dossier hors du profil, tout chemin passant par une jonction, et tout `MainBinaryName` qui n'est pas un simple nom de fichier. Si l'ancien exe est verrouillé (encore lancé), l'entrée HKCU est gardée et le nettoyage refait au démarrage suivant. Sortie dans la console : lignes `[legacy]`.

## Modifier le polling

`src-tauri/src/lib.rs` : `Duration::from_secs(2)` dans la boucle métriques.
