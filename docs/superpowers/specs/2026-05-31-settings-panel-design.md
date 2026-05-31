# Settings Panel — Design Spec

**Date:** 2026-05-31
**Statut:** Approuvé

## Objectif

Ajouter un panneau de paramètres accessible via le menu contextuel du widget. Ce panneau est une 2ème fenêtre Tauri sans barre de titre, au même style glassmorphism que le widget, qui permet de personnaliser l'apparence et le comportement du widget en live.

---

## Architecture

```
widget window  ←→  settings window   (deux WebViews, même app Tauri)
      ↓                    ↓
  plugin-store      plugin-store      ← même config.json, nouvelles clés
      ↑                    ↑
  Tauri event: "settings-changed"     ← settings émet, widget écoute
```

Les changements sont appliqués **immédiatement** (live preview). Pas de bouton "Appliquer" ni "Annuler".

---

## Nouvelles clés `config.json`

| Clé | Type | Valeur par défaut |
|---|---|---|
| `accentColor` | `"cyan" \| "matrix" \| "white" \| "windows"` | `"cyan"` |
| `transparency` | `"opaque" \| "glass" \| "ultra"` | `"glass"` |
| `showDetails` | `boolean` | `true` |
| `locked` | `boolean` | `false` |

---

## Fichiers

| Fichier | Action |
|---|---|
| `src-tauri/tauri.conf.json` | Déclarer la fenêtre `settings` (frameless, transparent, hidden au démarrage) |
| `src/routes/settings/+page.svelte` | **Nouveau** — UI du panneau |
| `src/lib/stores/settings.svelte.ts` | **Nouveau** — store partagé (charge/sauve via plugin-store, émet `settings-changed`) |
| `src/routes/+page.svelte` | Écouter `settings-changed`, appliquer theme class + locked |
| `src/lib/components/ContextMenu.svelte` | Ajouter bouton "Paramètres…" qui ouvre/toggle la fenêtre settings |
| `src/app.css` | Ajouter variables CSS des 4 thèmes et 3 niveaux de transparence |

---

## Fenêtre `settings`

- **Frameless + transparent** — même setup que le widget principal
- **Taille fixe** : ~250px × auto (fit-content)
- **Position** : s'ouvre à droite du widget si la place le permet, sinon à gauche. Calculé via `outerPosition()` + `outerSize()` du widget.
- **Fermeture** : clic sur backdrop invisible (même pattern que ContextMenu) ou bouton ✕ discret
- **N'apparaît pas dans la barre des tâches** (`skip_taskbar: true`)

---

## Sections du panneau

### Apparence

**Couleur d'accentuation** — 4 swatches cliquables :

| Option | Couleurs remplacées |
|---|---|
| Cyan Néon (défaut) | ok: `#06d6a0`, warn: `#ffd166`, danger: `#ff6b6b`, dl: `#74d7f7` |
| Vert Matrix | ok: `#00ff41`, warn: `#aaff00`, danger: `#ff4444`, dl: `#00cc33` |
| Blanc Épuré | ok: `#e8e8e8`, warn: `#cccccc`, danger: `#999999`, dl: `#ffffff` |
| Thème Windows | Couleur récupérée via commande Tauri Rust `get_accent_color` (registre Windows) |

Implémentation : classe CSS sur `<html>` (ex: `theme-matrix`). Les variables `--color-ok/warn/danger/dl` sont redéfinies par thème dans `app.css`.

**Transparence** — 3 options radio :

| Option | `--glass-bg` |
|---|---|
| Opaque | `rgba(10, 10, 10, 0.96)` |
| Glassmorphism (défaut) | `rgba(10, 10, 10, 0.78)` |
| Ultra-transparent | `rgba(10, 10, 10, 0.40)` |

**Afficher les détails** — toggle (boolean). Quand désactivé : classe `hide-details` sur `.widget`, les `.sub-line` passent à `display: none`. Le widget devient plus compact automatiquement.

### Comportement

**Verrouiller la position** — toggle. Quand actif : `onWidgetMouseDown` dans `+page.svelte` skippt `startDragging()`. Un indicateur visuel discret (ex: icône 🔒 en bas du panneau settings) rappelle que le lock est actif.

**Ancrer le widget** — 4 boutons en grille 2×2 :

```
↖ Haut gauche  |  ↗ Haut droite
↙ Bas gauche   |  ↘ Bas droite
```

Au clic : `currentMonitor()` → calcule la position physique du coin cible (en tenant compte de la taille du widget + scale factor) → `appWindow.setPosition()` → sauvegarde la position dans le store. Fonctionne sur multi-écrans : ancre toujours sur le moniteur où le widget se trouve au moment du clic.

---

## Flux de données

```
[User clique sur swatch "Matrix"]
  → settings store: accentColor = "matrix"
  → plugin-store.set("accentColor", "matrix") + save()
  → invoke Tauri event "settings-changed" { accentColor: "matrix" }
  → widget +page.svelte listener: document.documentElement.className = "theme-matrix"
  → CSS vars redéfinies → couleurs mises à jour immédiatement
```

---

## Commande Rust requise

`get_accent_color` — lit `HKCU\Software\Microsoft\Windows\DWM\AccentColor` (DWORD ABGR) et retourne la couleur hex CSS (`#rrggbb`). Uniquement appelée quand le thème "Thème Windows" est sélectionné.

---

## Ce qui n'est PAS inclus

- Bump de version / tag git / release — à faire après validation locale
- Sélecteur de moniteur dans le panneau (YAGNI — on ancre sur le moniteur courant)
- Slider de transparence (3 presets suffisent pour la v1)
- Animations d'ouverture du panneau
