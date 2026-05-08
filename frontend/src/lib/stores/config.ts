import { writable } from 'svelte/store';

export type LlmProviderName = 'ollama' | 'openai' | 'anthropic' | null;
export type EmbedProviderName = 'fastembed' | 'openai' | 'ollama' | null;

export interface AppConfig {
  advanced_enabled: boolean;
  llm_available: boolean;
  llm_provider: LlmProviderName;
  embed_available: boolean;
  embed_provider: EmbedProviderName;
  reranker_available: boolean;
}

export const appConfig = writable<AppConfig>({
  advanced_enabled: true,
  llm_available: false,
  llm_provider: null,
  embed_available: false,
  embed_provider: null,
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
