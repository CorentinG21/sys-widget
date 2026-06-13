import { load } from '@tauri-apps/plugin-store';
import { invoke } from '@tauri-apps/api/core';
import type { Lang } from '$lib/i18n';

export type AccentColor = 'cyan' | 'matrix' | 'white' | 'custom';
export type { Lang };

/** Transparency: 0–100 (opacity %). Default 78. */
export type Transparency = number;

export type DisplayMode = 0 | 1;

export interface Settings {
  accentColor:  AccentColor;
  customColor:  string;        // hex for 'custom' theme, e.g. "#c084fc"
  transparency: Transparency;  // 20–98
  displayMode:  DisplayMode;   // 0=compact, 1=normal, 2=full
  locked:       boolean;
  // Visible rows
  showCpu:      boolean;
  showGpu:      boolean;
  showRam:      boolean;
  showDisks:    boolean;
  showNetwork:  boolean;
  // Polling interval in seconds
  pollInterval: 1 | 2 | 5;
  // Window layer
  alwaysOnTop: boolean;
  // Temperature unit
  tempUnit: 'C' | 'F' | 'K';
  // UI language
  lang: Lang;
}

const STORE_PATH = 'config.json';

export const settings = $state<Settings>({
  accentColor:  'cyan',
  customColor:  '#c084fc',
  transparency: 78,
  displayMode:  1,
  locked:       false,
  showCpu:      true,
  showGpu:      true,
  showRam:      true,
  showDisks:    true,
  showNetwork:  true,
  pollInterval: 2,
  alwaysOnTop: false,
  tempUnit: 'C',
  lang: 'fr',
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
  // Migrate legacy showDetails boolean → displayMode number
  const savedMode = await store.get<number>('displayMode');
  const legacyDetails = await store.get<boolean>('showDetails');
  const rawMode = (savedMode === 0 || savedMode === 1 || savedMode === 2) ? savedMode : (legacyDetails === false ? 0 : 1);
  settings.displayMode = rawMode === 2 ? 1 : rawMode as DisplayMode;
  settings.locked       = (await store.get<boolean>('locked'))      ?? false;
  settings.showCpu      = (await store.get<boolean>('showCpu'))     ?? true;
  settings.showGpu      = (await store.get<boolean>('showGpu'))     ?? true;
  settings.showRam      = (await store.get<boolean>('showRam'))     ?? true;
  settings.showDisks    = (await store.get<boolean>('showDisks'))   ?? true;
  settings.showNetwork  = (await store.get<boolean>('showNetwork')) ?? true;
  const savedInterval   = await store.get<number>('pollInterval');
  settings.pollInterval = ([1, 2, 5].includes(savedInterval as number)
    ? savedInterval : 2) as 1 | 2 | 5;
  settings.alwaysOnTop  = (await store.get<boolean>('alwaysOnTop')) ?? false;
  const savedUnit = await store.get<string>('tempUnit');
  settings.tempUnit = (['C', 'F', 'K'].includes(savedUnit as string) ? savedUnit : 'C') as 'C' | 'F' | 'K';
  const savedLang = await store.get<string>('lang');
  settings.lang = (['fr', 'en'].includes(savedLang as string) ? savedLang : 'fr') as Lang;
}

export async function saveSettings(): Promise<void> {
  try {
    const store = await load(STORE_PATH);
    await store.set('accentColor',  settings.accentColor);
    await store.set('customColor',  settings.customColor);
    await store.set('transparency', settings.transparency);
    await store.set('displayMode',  settings.displayMode);
    await store.set('locked',       settings.locked);
    await store.set('showCpu',      settings.showCpu);
    await store.set('showGpu',      settings.showGpu);
    await store.set('showRam',      settings.showRam);
    await store.set('showDisks',    settings.showDisks);
    await store.set('showNetwork',  settings.showNetwork);
    await store.set('pollInterval', settings.pollInterval);
    await store.set('alwaysOnTop',  settings.alwaysOnTop);
    await store.set('tempUnit',     settings.tempUnit);
    await store.set('lang',         settings.lang);
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

  // Display mode
  html.dataset.displayMode = String(settings.displayMode);
}
