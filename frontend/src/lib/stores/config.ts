import { writable } from 'svelte/store';

export interface AppConfig {
  advanced_enabled: boolean;
  ollama_available: boolean;
  reranker_available: boolean;
}

export const appConfig = writable<AppConfig>({
  advanced_enabled: true,
  ollama_available: false,
  reranker_available: false,
});

let loaded = false;
export async function loadAppConfig() {
  if (loaded) return;
  loaded = true;
  try {
    const res = await fetch('/api/config');
    if (res.ok) appConfig.set(await res.json());
  } catch (e) {
    console.warn('Failed to load app config:', e);
  }
}
