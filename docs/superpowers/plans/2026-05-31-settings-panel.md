# Settings Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ajouter un panneau de paramètres glassmorphism (fenêtre Tauri séparée, sans barre de titre) ouvert via le menu contextuel, permettant de changer le thème de couleur, la transparence, masquer les détails et ancrer le widget dans un coin d'écran.

**Architecture:** La fenêtre settings est une 2ème WebView Tauri frameless/transparent, même style que le widget. Les changements sont appliqués en live via l'événement Tauri `settings-changed` émis depuis la fenêtre settings et écouté par la fenêtre widget. Les préférences sont persistées dans `config.json` (même store que la position).

**Tech Stack:** Svelte 5 runes, SvelteKit static adapter, Tauri v2, tauri-plugin-store, CSS data-attributes pour les thèmes.

---

## Fichiers

| Fichier | Action |
|---|---|
| `src/app.css` | Modifier — ajouter vars CSS thèmes + transparence + hide-details |
| `src/lib/utils/colors.ts` | Modifier — retourner `var(--color-*)` au lieu de hex hardcodés |
| `src/lib/stores/settings.svelte.ts` | **Créer** — store partagé : load/save/emit |
| `src-tauri/tauri.conf.json` | Modifier — ajouter `"label": "main"` + déclarer fenêtre `settings` |
| `src-tauri/src/lib.rs` | Modifier — ajouter commande `get_accent_color` |
| `src/routes/settings/+page.svelte` | **Créer** — UI du panneau de paramètres |
| `src/routes/+page.svelte` | Modifier — charger settings au démarrage + écouter `settings-changed` |
| `src/lib/components/ContextMenu.svelte` | Modifier — ajouter bouton "Paramètres…" |

---

## Task 1: CSS — système de thèmes et transparence

**Files:**
- Modify: `src/app.css`

- [ ] **Step 1: Ajouter les vars de couleur dans `:root` et les thèmes**

Ajouter à la fin de `src/app.css` (après `.divider`) :

```css
/* ── Color vars (overridden by data-theme attribute) ── */
:root {
  --color-ok:     #06d6a0;
  --color-warn:   #ffd166;
  --color-danger: #ff6b6b;
  --color-dl:     #74d7f7;
}

[data-theme="matrix"] {
  --color-ok:     #00ff41;
  --color-warn:   #aaff00;
  --color-danger: #ff4444;
  --color-dl:     #00cc33;
}

[data-theme="white"] {
  --color-ok:     #e8e8e8;
  --color-warn:   #cccccc;
  --color-danger: #999999;
  --color-dl:     #ffffff;
}

/* Windows theme: accent color injected via JS as --windows-accent */
[data-theme="windows"] {
  --color-ok: var(--windows-accent, #06d6a0);
  --color-dl: var(--windows-accent, #74d7f7);
}

/* ── Transparency levels ── */
[data-transparency="opaque"] { --glass-bg: rgba(10, 10, 10, 0.96); }
[data-transparency="ultra"]  { --glass-bg: rgba(10, 10, 10, 0.40); }
/* "glass" = default in :root, no selector needed */

/* ── Hide details toggle ── */
[data-hide-details] .sub-line {
  display: none;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/app.css
git commit -m "feat: CSS theme + transparency + hide-details data-attribute system"
```

---

## Task 2: Mettre à jour colors.ts pour utiliser les CSS vars

**Files:**
- Modify: `src/lib/utils/colors.ts`

- [ ] **Step 1: Remplacer les hex hardcodés par des CSS vars dans thresholdColor et netColors**

```typescript
/**
 * Returns a CSS color var for a 0–100% usage value.
 */
export function thresholdColor(percent: number): string {
  if (percent >= 90) return 'var(--color-danger)';
  if (percent >= 70) return 'var(--color-warn)';
  return 'var(--color-ok)';
}

/**
 * Returns upload and download color vars for a network rate in bytes/s.
 */
export function netColors(bytesPerSec: number): { upload: string; download: string } {
  const mbps = bytesPerSec / 1_048_576;
  if (mbps >= 10) return { upload: 'var(--color-danger)', download: 'var(--color-danger)' };
  if (mbps >= 1)  return { upload: 'var(--color-warn)',   download: 'var(--color-warn)' };
  return { upload: 'var(--color-ok)', download: 'var(--color-dl)' };
}
```

Laisser `formatRate` et `formatBytes` inchangés.

- [ ] **Step 2: Vérifier que le widget s'affiche toujours correctement**

Lancer `npm run tauri dev` en terminal admin et vérifier que les barres et textes colorés s'affichent normalement (cyan par défaut).

- [ ] **Step 3: Commit**

```bash
git add src/lib/utils/colors.ts
git commit -m "refactor: colors.ts returns CSS vars instead of hardcoded hex"
```

---

## Task 3: Settings store

**Files:**
- Create: `src/lib/stores/settings.svelte.ts`

- [ ] **Step 1: Créer le store**

```typescript
import { load } from '@tauri-apps/plugin-store';
import { emit } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';

export type AccentColor = 'cyan' | 'matrix' | 'white' | 'windows';
export type Transparency = 'opaque' | 'glass' | 'ultra';

export interface Settings {
  accentColor: AccentColor;
  transparency: Transparency;
  showDetails: boolean;
  locked: boolean;
}

const STORE_PATH = 'config.json';

export const settings = $state<Settings>({
  accentColor: 'cyan',
  transparency: 'glass',
  showDetails: true,
  locked: false,
});

export async function loadSettings(): Promise<void> {
  const store = await load(STORE_PATH);
  settings.accentColor = (await store.get<AccentColor>('accentColor')) ?? 'cyan';
  settings.transparency = (await store.get<Transparency>('transparency')) ?? 'glass';
  settings.showDetails  = (await store.get<boolean>('showDetails'))  ?? true;
  settings.locked       = (await store.get<boolean>('locked'))       ?? false;
}

export async function saveAndEmit(): Promise<void> {
  const store = await load(STORE_PATH);
  await store.set('accentColor', settings.accentColor);
  await store.set('transparency', settings.transparency);
  await store.set('showDetails',  settings.showDetails);
  await store.set('locked',       settings.locked);
  await store.save();
  await emit('settings-changed', { ...settings });
}

/** Apply theme + transparency + hide-details to the document. */
export async function applyToDocument(): Promise<void> {
  const html = document.documentElement;

  // Theme
  if (settings.accentColor === 'windows') {
    const hex = await invoke<string>('get_accent_color');
    html.style.setProperty('--windows-accent', hex);
  }
  html.dataset.theme = settings.accentColor;

  // Transparency
  html.dataset.transparency = settings.transparency;

  // Details
  if (settings.showDetails) {
    delete html.dataset.hideDetails;
  } else {
    html.dataset.hideDetails = '';
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/stores/settings.svelte.ts
git commit -m "feat: settings store with load/save/emit/applyToDocument"
```

---

## Task 4: Tauri config + commande Rust get_accent_color

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Ajouter `"label": "main"` à la fenêtre existante et déclarer la fenêtre settings**

Dans `src-tauri/tauri.conf.json`, remplacer le tableau `"windows"` :

```json
"windows": [
  {
    "label": "main",
    "title": "SysmonWidget",
    "width": 320,
    "height": 600,
    "decorations": false,
    "transparent": true,
    "skipTaskbar": true,
    "shadow": false,
    "resizable": false,
    "visible": false
  },
  {
    "label": "settings",
    "url": "/settings",
    "title": "SysmonWidget Settings",
    "width": 260,
    "height": 400,
    "decorations": false,
    "transparent": true,
    "skipTaskbar": true,
    "shadow": false,
    "resizable": false,
    "visible": false,
    "alwaysOnTop": true
  }
]
```

- [ ] **Step 2: Ajouter la commande `get_accent_color` dans lib.rs**

Ajouter après `install_update` (avant `// ─── App setup`) :

```rust
/// Read the current Windows accent color from the registry.
/// Returns a CSS hex string like "#0078d4". Falls back to "#06d6a0" on error.
#[tauri::command]
fn get_accent_color() -> String {
    #[cfg(target_os = "windows")]
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut cmd = std::process::Command::new("powershell");
    cmd.args([
        "-NoProfile",
        "-NonInteractive",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        "(Get-ItemProperty 'HKCU:\\Software\\Microsoft\\Windows\\DWM' \
         -Name AccentColor -ErrorAction SilentlyContinue).AccentColor",
    ]);

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    if let Ok(output) = cmd.output() {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if let Ok(val) = s.parse::<i64>() {
                let val = val as u32;
                let r = (val & 0xFF) as u8;
                let g = ((val >> 8) & 0xFF) as u8;
                let b = ((val >> 16) & 0xFF) as u8;
                return format!("#{:02x}{:02x}{:02x}", r, g, b);
            }
        }
    }
    "#06d6a0".to_string()
}
```

- [ ] **Step 3: Enregistrer la commande dans `invoke_handler!`**

Dans `tauri::generate_handler![]`, ajouter `get_accent_color` :

```rust
.invoke_handler(tauri::generate_handler![
    restart_app,
    quit_app,
    startup_is_registered,
    startup_toggle,
    check_update,
    install_update,
    get_accent_color,
])
```

- [ ] **Step 4: Vérifier que ça compile**

```bash
cd src-tauri && cargo check
```

Attendre la fin sans erreur.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/tauri.conf.json src-tauri/src/lib.rs
git commit -m "feat: declare settings window + get_accent_color Rust command"
```

---

## Task 5: Page settings (UI du panneau)

**Files:**
- Create: `src/routes/settings/+page.svelte`

- [ ] **Step 1: Créer la page**

```svelte
<script lang="ts">
  import '../../app.css';
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { load } from '@tauri-apps/plugin-store';
  import {
    settings,
    loadSettings,
    saveAndEmit,
    applyToDocument,
    type AccentColor,
    type Transparency,
  } from '$lib/stores/settings.svelte';

  const win = getCurrentWindow();

  onMount(async () => {
    await loadSettings();
    await applyToDocument();
  });

  async function setAccent(val: AccentColor) {
    settings.accentColor = val;
    await applyToDocument();
    await saveAndEmit();
  }

  async function setTransparency(val: Transparency) {
    settings.transparency = val;
    await applyToDocument();
    await saveAndEmit();
  }

  async function toggleDetails() {
    settings.showDetails = !settings.showDetails;
    await applyToDocument();
    await saveAndEmit();
  }

  async function toggleLocked() {
    settings.locked = !settings.locked;
    await saveAndEmit();
  }

  async function anchorTo(corner: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right') {
    const mainWin = await WebviewWindow.getByLabel('main');
    if (!mainWin) return;

    const monitor = await mainWin.currentMonitor();
    const widgetSize = await mainWin.outerSize();
    if (!monitor || !widgetSize) return;

    const mx = monitor.position.x;
    const my = monitor.position.y;
    const mw = monitor.size.width;
    const mh = monitor.size.height;
    const margin = 12;

    let x: number, y: number;
    if (corner === 'top-left')     { x = mx + margin;                         y = my + margin; }
    else if (corner === 'top-right')    { x = mx + mw - widgetSize.width - margin;  y = my + margin; }
    else if (corner === 'bottom-left')  { x = mx + margin;                         y = my + mh - widgetSize.height - margin; }
    else                                { x = mx + mw - widgetSize.width - margin;  y = my + mh - widgetSize.height - margin; }

    await mainWin.setPosition({ type: 'Physical', x, y });

    // Persist position in the same store
    const store = await load('config.json');
    await store.set('position', { x, y });
    await store.save();
  }

  function close() { win.hide(); }

  const ACCENTS: { id: AccentColor; label: string; color: string }[] = [
    { id: 'cyan',    label: 'Cyan Néon',      color: '#06d6a0' },
    { id: 'matrix',  label: 'Vert Matrix',    color: '#00ff41' },
    { id: 'white',   label: 'Blanc Épuré',    color: '#e0e0e0' },
    { id: 'windows', label: 'Thème Windows',  color: '#0078d4' },
  ];

  const TRANSPARENCIES: { id: Transparency; label: string }[] = [
    { id: 'opaque', label: 'Opaque' },
    { id: 'glass',  label: 'Glassmorphism' },
    { id: 'ultra',  label: 'Ultra-transparent' },
  ];
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="panel" onmousedown={(e) => { if (e.button === 0) win.startDragging(); }}>

  <div class="header">
    <span class="title">⚙ Paramètres</span>
    <button class="close-btn" onclick={close}>✕</button>
  </div>

  <!-- ── Couleur d'accentuation ── -->
  <section>
    <div class="section-label">Couleur</div>
    <div class="swatches">
      {#each ACCENTS as accent}
        <button
          class="swatch"
          class:active={settings.accentColor === accent.id}
          style="background: {accent.color};"
          title={accent.label}
          onclick={() => setAccent(accent.id)}
        ></button>
      {/each}
    </div>
  </section>

  <div class="divider"></div>

  <!-- ── Transparence ── -->
  <section>
    <div class="section-label">Transparence</div>
    <div class="radio-group">
      {#each TRANSPARENCIES as t}
        <button
          class="radio-btn"
          class:active={settings.transparency === t.id}
          onclick={() => setTransparency(t.id)}
        >
          {t.label}
        </button>
      {/each}
    </div>
  </section>

  <div class="divider"></div>

  <!-- ── Toggles ── -->
  <section>
    <button class="toggle-row" onclick={toggleDetails}>
      <span>{settings.showDetails ? '✓' : '○'} Afficher les détails</span>
    </button>
    <button class="toggle-row" onclick={toggleLocked}>
      <span>{settings.locked ? '✓' : '○'} Verrouiller la position</span>
    </button>
  </section>

  <div class="divider"></div>

  <!-- ── Ancrage ── -->
  <section>
    <div class="section-label">Ancrer le widget</div>
    <div class="anchor-grid">
      <button class="anchor-btn" onclick={() => anchorTo('top-left')}>↖ Haut gauche</button>
      <button class="anchor-btn" onclick={() => anchorTo('top-right')}>↗ Haut droite</button>
      <button class="anchor-btn" onclick={() => anchorTo('bottom-left')}>↙ Bas gauche</button>
      <button class="anchor-btn" onclick={() => anchorTo('bottom-right')}>↘ Bas droite</button>
    </div>
  </section>

</div>

<style>
  html, body {
    width: 100%;
    height: 100%;
    background: transparent;
    overflow: hidden;
    user-select: none;
  }

  .panel {
    font-family: 'Consolas', 'Cascadia Code', 'Courier New', monospace;
    font-size: 12px;
    background: rgba(12, 12, 12, 0.92);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
    border: 1px solid rgba(255, 255, 255, 0.09);
    border-radius: 12px;
    padding: 10px 14px 14px;
    color: #e8e8e8;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-height: 100vh;
    box-sizing: border-box;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 2px;
  }

  .title {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.45);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.3);
    font-size: 12px;
    cursor: pointer;
    padding: 0 2px;
    font-family: inherit;
    pointer-events: auto;
    transition: color 0.15s;
  }

  .close-btn:hover { color: #ff6b6b; }

  .section-label {
    font-size: 9px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: rgba(255, 255, 255, 0.3);
    margin-bottom: 6px;
  }

  section {
    display: flex;
    flex-direction: column;
  }

  /* Swatches */
  .swatches {
    display: flex;
    gap: 8px;
  }

  .swatch {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    pointer-events: auto;
    transition: border-color 0.15s, transform 0.1s;
  }

  .swatch.active {
    border-color: rgba(255, 255, 255, 0.8);
    transform: scale(1.15);
  }

  /* Transparency radio */
  .radio-group {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .radio-btn {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.45);
    font-family: inherit;
    font-size: 11px;
    text-align: left;
    padding: 3px 0;
    cursor: pointer;
    pointer-events: auto;
    transition: color 0.15s;
  }

  .radio-btn.active { color: #e8e8e8; }
  .radio-btn.active::before { content: '● '; }
  .radio-btn:not(.active)::before { content: '○ '; }

  /* Toggle rows */
  .toggle-row {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.6);
    font-family: inherit;
    font-size: 11px;
    text-align: left;
    padding: 4px 0;
    cursor: pointer;
    pointer-events: auto;
    transition: color 0.15s;
    width: 100%;
  }

  .toggle-row:hover { color: #e8e8e8; }

  /* Anchor grid */
  .anchor-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
  }

  .anchor-btn {
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 5px;
    color: rgba(255, 255, 255, 0.55);
    font-family: inherit;
    font-size: 10px;
    padding: 5px 6px;
    cursor: pointer;
    pointer-events: auto;
    text-align: left;
    transition: background 0.15s, color 0.15s;
  }

  .anchor-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: #e8e8e8;
  }

  .divider {
    height: 1px;
    background: rgba(255, 255, 255, 0.07);
  }
</style>
```

- [ ] **Step 2: Vérifier que la route est accessible**

Lancer `npm run dev` et ouvrir `http://localhost:1420/settings` dans un navigateur. Le panneau doit s'afficher avec ses sections.

- [ ] **Step 3: Commit**

```bash
git add src/routes/settings/+page.svelte
git commit -m "feat: settings panel UI — color swatches, transparency, toggles, anchor"
```

---

## Task 6: Intégration widget — charger les settings et écouter les changements

**Files:**
- Modify: `src/routes/+page.svelte`

- [ ] **Step 1: Importer le store**

Dans la section `<script lang="ts">`, ajouter après les imports existants (`listen` est déjà importé depuis `@tauri-apps/api/event`) :

```typescript
import { settings, loadSettings, applyToDocument, type Settings } from '$lib/stores/settings.svelte';
```

- [ ] **Step 2: Charger et appliquer les settings dans `onMount`**

Dans `onMount`, après `await startListening();`, ajouter :

```typescript
await loadSettings();
await applyToDocument();

await listen<Settings>('settings-changed', async (event) => {
  settings.accentColor = event.payload.accentColor;
  settings.transparency = event.payload.transparency;
  settings.showDetails  = event.payload.showDetails;
  settings.locked       = event.payload.locked;
  await applyToDocument();
});
```

- [ ] **Step 3: Utiliser `settings.locked` dans `onWidgetMouseDown`**

Remplacer la fonction `onWidgetMouseDown` existante par :

```typescript
function onWidgetMouseDown(e: MouseEvent) {
  if (e.button !== 0) return;
  if (settings.locked) return;
  const target = e.target as HTMLElement;
  if (target.closest('button, input, a, [role="button"]')) return;
  appWindow.startDragging();
}
```

- [ ] **Step 4: Vérifier le comportement**

Lancer `npm run tauri dev` en admin. Au démarrage le thème cyan doit s'afficher. Modifier manuellement `config.json` (dans `%APPDATA%\sysmon-widget\`) pour mettre `"accentColor": "matrix"`, relancer — les barres doivent être vertes.

- [ ] **Step 5: Commit**

```bash
git add src/routes/+page.svelte
git commit -m "feat: widget loads and reacts to settings (theme, transparency, locked, details)"
```

---

## Task 7: Bouton "Paramètres…" dans le menu contextuel

**Files:**
- Modify: `src/lib/components/ContextMenu.svelte`

- [ ] **Step 1: Ajouter l'import WebviewWindow et la fonction openSettings**

Dans `<script lang="ts">`, après les imports existants :

```typescript
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { getCurrentWindow } from '@tauri-apps/api/window';

async function openSettings() {
  onclose();
  const mainWin = getCurrentWindow();
  const pos  = await mainWin.outerPosition();
  const size = await mainWin.outerSize();

  const settingsWin = await WebviewWindow.getByLabel('settings');
  if (!settingsWin) return;

  // Position: à droite du widget si possible, sinon à gauche
  const settingsWidth = 260;
  const gap = 8;
  const targetX = pos.x + size.width + gap;

  await settingsWin.setPosition({ type: 'Physical', x: targetX, y: pos.y });
  await settingsWin.show();
  await settingsWin.setFocus();
}
```

- [ ] **Step 2: Ajouter le bouton dans le template**

Dans le template `<div class="context-menu">`, juste avant la première `<div class="menu-divider">` après la version, ajouter :

```svelte
<button class="menu-item" role="menuitem" onclick={openSettings}>
  ⚙ Paramètres…
</button>
```

Le bloc concerné devient :

```svelte
<div class="menu-version" role="menuitem" aria-disabled="true">
  SysmonWidget v{version}
</div>

<div class="menu-divider"></div>

<button class="menu-item" role="menuitem" onclick={openSettings}>
  ⚙ Paramètres…
</button>

<div class="menu-divider"></div>

{#if updateVersion}
  ...
```

- [ ] **Step 3: Vérifier end-to-end**

Lancer `npm run tauri dev` en admin. Clic droit sur le widget → "⚙ Paramètres…" → le panneau s'ouvre à droite. Changer le thème → le widget se met à jour immédiatement. Cliquer ✕ → le panneau se ferme.

Tester aussi :
- Verrouiller la position → clic-glisser ne déplace plus le widget
- Ancrer "Haut droite" → le widget snape dans le coin

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/ContextMenu.svelte
git commit -m "feat: context menu opens settings panel"
```
