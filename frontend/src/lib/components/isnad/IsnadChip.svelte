<script lang="ts">
  import type { ApiNarratorSearchResult } from '$lib/types';
  import { language } from '$lib/stores/language';
  import Badge from '$lib/components/common/Badge.svelte';

  let { narrator, onRemove }: {
    narrator: ApiNarratorSearchResult;
    onRemove: () => void;
  } = $props();

  let displayName = $derived(
    $language === 'en' && narrator.name_en ? narrator.name_en : (narrator.name_ar || narrator.name_en)
  );
</script>

<span class="isnad-chip">
  <a href="/narrators/{narrator.id}" class="chip-name">{displayName}</a>
  {#if narrator.generation}<Badge text={narrator.generation} variant="accent" />{/if}
  <button class="chip-remove" onclick={onRemove} title="Remove">&times;</button>
</span>

<style>
  .isnad-chip { display: inline-flex; align-items: center; gap: 6px; padding: 4px 8px 4px 12px; background: var(--accent-muted); border-radius: 20px; font-size: 0.8rem; }
  .chip-name { color: var(--accent); font-weight: 500; text-decoration: none; max-width: 200px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .chip-name:hover { text-decoration: underline; color: var(--accent); }
  .chip-remove { display: flex; align-items: center; justify-content: center; width: 18px; height: 18px; border: none; background: none; color: var(--text-muted); font-size: 1rem; cursor: pointer; border-radius: 50%; transition: all var(--transition); padding: 0; line-height: 1; }
  .chip-remove:hover { background: var(--error); color: white; }
</style>
