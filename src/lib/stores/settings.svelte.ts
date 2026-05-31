import { load } from '@tauri-apps/plugin-store';
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

export async function saveSettings(): Promise<void> {
  try {
    const store = await load(STORE_PATH);
    await store.set('accentColor', settings.accentColor);
    await store.set('transparency', settings.transparency);
    await store.set('showDetails',  settings.showDetails);
    await store.set('locked',       settings.locked);
    await store.save();
  } catch (e) {
    console.error('[settings] save failed:', e);
  }
}

/** Apply current settings to the document (CSS vars + data-attributes). */
export async function applyToDocument(): Promise<void> {
  const html = document.documentElement;

  if (settings.accentColor === 'windows') {
    const hex = await invoke<string>('get_accent_color');
    html.style.setProperty('--windows-accent', hex);
  }
  html.dataset.theme = settings.accentColor;
  html.dataset.transparency = settings.transparency;

  if (settings.showDetails) {
    delete html.dataset.hideDetails;
  } else {
    html.dataset.hideDetails = '';
  }
}
