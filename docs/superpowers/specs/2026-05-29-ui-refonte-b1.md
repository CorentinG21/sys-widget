# SysmonWidget — Refonte UI B1

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Refondre le layout de toutes les lignes de métriques selon le style B1 — ultra compact, deux niveaux (ligne principale + sous-ligne).

**Architecture:** Chaque MetricRow, DiskRow et NetworkRow est refondu. Les tokens CSS sont mis à jour. Aucun changement backend.

**Fichiers touchés:** `src/app.css`, `src/lib/components/MetricRow.svelte`, `src/lib/components/DiskRow.svelte`, `src/lib/components/NetworkRow.svelte`

---

## Design B1 — Spécification exacte

### Ligne principale (MetricRow, DiskRow)

```
[LABEL 36px] [BAR flex:1 h:4px] [% 28px right-aligned]
```

- **Grid** : `grid-template-columns: 36px 1fr 28px; gap: 6px; align-items: center`
- **Label** : `font-size: 10px`, uppercase, `letter-spacing: 0.05em`, `color: rgba(255,255,255,0.45)`
- **Barre** : hauteur 4px, `border-radius: 2px`, fond `rgba(255,255,255,0.08)`
- **%** : `font-size: 11px`, `text-align: right`, couleur dynamique (`thresholdColor`)

### Sous-ligne (détails secondaires)

```
[indent 42px] [détails : °C · GB · top process]
```

- **Indentation** : `padding-left: 42px` (= 36px label + 6px gap)
- **Font-size** : `9px`
- **Couleurs** :
  - Température °C : `rgba(255,255,255,0.28)`
  - GB used/total : `rgba(255,255,255,0.28)`
  - Top process (🔥 nom · %) : `rgba(255,255,255,0.18)`
- **Margin** : `2px 0 7px` (serré au-dessus, léger espace en dessous)
- **Overflow** : `text-overflow: ellipsis; white-space: nowrap; overflow: hidden` sur le nom du top process

### Lignes sans sous-ligne

RAM, disques sans info extra → pas de sous-ligne vide. On affiche la sous-ligne uniquement si elle a du contenu.

### DiskRow

Même grid que MetricRow. Sous-ligne : `used / total GB` en 9px muted.

```
C:   [████████░]   82%
     376 / 446 GB
```

### NetworkRow

Pas de barre. Une seule ligne :

```
NET   ↑ 5.8 KB/s   ↓ 2.4 KB/s
```

- Label `NET` 36px, même style uppercase
- Upload : `color: var(--color-ok)` (#06d6a0)
- Download : `color: var(--color-dl)` (#74d7f7)
- Seuils de couleur réseau conservés (< 1 MB/s : vert/cyan, 1–9 : jaune, ≥ 10 : rouge)

### Top Process (sous-ligne CPU)

```
     🔥 msedgewebview2.exe · 1%
```

- S'affiche uniquement si `metrics.top_cpu !== null`
- Même indentation 42px, 9px, `rgba(255,255,255,0.18)`
- Sur la même sous-ligne que la temp CPU : `47°C · 🔥 nom · 1%`
  - ou séparé si trop long (overflow ellipsis)

### Tokens CSS à mettre à jour

```css
--label-width: 36px;   /* était 54px */
--bar-height:  4px;    /* inchangé */
--row-gap:     6px;    /* inchangé */
```

Nouveaux tokens :

```css
--subline-size:   9px;
--subline-color:  rgba(255, 255, 255, 0.28);
--subline-indent: 42px;   /* label + gap */
--subline-margin: 2px 0 7px;
```

### État N/A (GPU non détecté)

Même ligne principale avec `%` affiché `N/A` en `rgba(255,255,255,0.25)`. Pas de sous-ligne.

---

## Ce qui ne change PAS

- Glassmorphism `.widget` (fond, blur, border, radius)
- Palette couleurs (`--color-ok/warn/danger/dl`)
- Sparkline au survol (overlay absolu, pas affecté par le layout)
- UpdateBanner, ContextMenu
- Tout le backend Rust
