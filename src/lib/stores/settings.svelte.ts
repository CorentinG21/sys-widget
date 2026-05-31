import { load } from '@tauri-apps/plugin-store';
import { invoke } from '@tauri-apps/api/core';

export type AccentColor = 'cyan' | 'matrix' | 'white' | 'custom';

/** Transparency: 0–100 (opacity %). Default 78. */
export type Transparency = number;

export interface Settings {
  accentColor:  AccentColor;
  customColor:  string;        // hex for 'custom' theme, e.g. "#c084fc"
  transparency: Transparency;  // 20–98
  showDetails:  boolean;
  locked:       boolean;
}

const STORE_PATH = 'config.json';

export const settings = $state<Settings>({
  accentColor:  'cyan',
  customColor:  '#c084fc',
  transparency: 78,
  showDetails:  true,
  locked:       false,
});

/** Migrate old string values → numeric. */
function migrateTransparency(val: unknown): number {
  if (typeof val === 'number') return Math.min(98, Math.max(20, val));
  if (val === 'opaque') return 96;
  if (val === 'ultra')  return 40;
  return 78; // 'glass' or unknown
}

export async function loadSettings(): Promise<void> {
  const store = await load(STORE_PATH);
  const savedAccent = await store.get<string>('accentColor');
  // Migrate legacy values
  const migratedAccent = (savedAccent === 'windows' || savedAccent === 'neutral')
    ? 'cyan' : savedAccent as AccentColor;
  settings.accentColor  = migratedAccent ?? 'cyan';
  settings.customColor  = (await store.get<string>('customColor'))  ?? '#c084fc';
  settings.transparency = migrateTransparency(await store.get('transparency'));
  settings.showDetails  = (await store.get<boolean>('showDetails')) ?? true;
  settings.locked       = (await store.get<boolean>('locked'))      ?? false;
}

export async function saveSettings(): Promise<void> {
  try {
    const store = await load(STORE_PATH);
    await store.set('accentColor',  settings.accentColor);
    await store.set('customColor',  settings.customColor);
    await store.set('transparency', settings.transparency);
    await store.set('showDetails',  settings.showDetails);
    await store.set('locked',       settings.locked);
    await store.save();
  } catch (e) {
    console.error('[settings] save failed:', e);
  }
}

/** Apply current settings to the document. */
export async function applyToDocument(): Promise<void> {
  const html = document.documentElement;

  // Theme
  if (settings.accentColor === 'custom') {
    html.style.setProperty('--custom-accent', settings.customColor);
  }
  html.dataset.theme = settings.accentColor;

  // Transparency — set CSS var directly (no more data-attribute)
  html.style.setProperty(
    '--glass-bg',
    `rgba(10, 10, 10, ${(settings.transparency / 100).toFixed(2)})`
  );

  // Details
  if (settings.showDetails) {
    delete html.dataset.hideDetails;
  } else {
    html.dataset.hideDetails = '';
  }
}
