<script lang="ts">
  import { narratorAutocomplete } from '$lib/api';
  import type { ApiNarratorSearchResult } from '$lib/types';
  import { language } from '$lib/stores/language';
  import { bilingualDisplayName } from '$lib/normalize';
  import Badge from '$lib/components/common/Badge.svelte';
  import { onDestroy } from 'svelte';

  let { onSelect, excludeIds = [], placeholder = 'Search narrators...' }: {
    onSelect: (narrator: ApiNarratorSearchResult) => void;
    excludeIds?: string[];
    placeholder?: string;
  } = $props();

  let query = $state('');
  let suggestions: ApiNarratorSearchResult[] = $state([]);
  let loading = $state(false);
  let showDropdown = $state(false);
  let selectedIndex = $state(-1);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  onDestroy(() => { if (debounceTimer) clearTimeout(debounceTimer); });

  function onInput() {
    const q = query.trim();
    if (q.length < 2) {
      suggestions = [];
      showDropdown = false;
      return;
    }
    if (debounceTimer) clearTimeout(debounceTimer);
    const searchQuery = q;
    debounceTimer = setTimeout(async () => {
      loading = true;
      try {
        const res = await narratorAutocomplete(searchQuery, 8);
        if (query.trim() === searchQuery) {
          suggestions = res.filter(n => !excludeIds.includes(n.id));
          showDropdown = suggestions.length > 0;
          selectedIndex = -1;
        }
      } catch { /* ignore */ }
      finally { loading = false; }
    }, 200);
  }

  function select(n: ApiNarratorSearchResult) {
    onSelect(n);
    query = '';
    suggestions = [];
    showDropdown = false;
  }

  function onKeydown(e: KeyboardEvent) {
    if (!showDropdown) return;
    if (e.key === 'ArrowDown') { e.preventDefault(); selectedIndex = Math.min(selectedIndex + 1, suggestions.length - 1); }
    else if (e.key === 'ArrowUp') { e.preventDefault(); selectedIndex = Math.max(selectedIndex - 1, 0); }
    else if (e.key === 'Enter' && selectedIndex >= 0) { e.preventDefault(); select(suggestions[selectedIndex]); }
    else if (e.key === 'Escape') { showDropdown = false; }
  }
</script>

<div class="autocomplete-wrapper">
  <input type="text" {placeholder} bind:value={query} oninput={onInput} onkeydown={onKeydown}
         onfocus={() => { if (suggestions.length > 0) showDropdown = true; }}
         onblur={() => setTimeout(() => { showDropdown = false; }, 150)}
         class="autocomplete-input" />
  {#if showDropdown}
    <div class="dropdown">
      {#each suggestions as s, i (s.id)}
        <button class="dropdown-item" class:selected={i === selectedIndex}
                onmousedown={(e) => { e.preventDefault(); select(s); }}>
          <span class="item-name">
            {bilingualDisplayName(s, $language, s.id)}
          </span>
          {#if s.generation}<Badge text={s.generation} variant="accent" />{/if}
          {#if s.hadith_count}<span class="item-count mono">{s.hadith_count}</span>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .autocomplete-wrapper { position: relative; }
  .autocomplete-input { width: 100%; }
  .dropdown { position: absolute; top: 100%; left: 0; right: 0; z-index: 50; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); margin-top: 4px; max-height: 300px; overflow-y: auto; box-shadow: 0 4px 12px rgba(0,0,0,0.1); }
  .dropdown-item { display: flex; align-items: center; gap: 8px; width: 100%; padding: 10px 12px; background: none; border: none; color: var(--text-primary); font-size: 0.85rem; cursor: pointer; transition: background var(--transition); text-align: left; }
  .dropdown-item:hover, .dropdown-item.selected { background: var(--bg-hover); }
  .item-name { flex: 1; }
  .item-count { color: var(--text-muted); font-size: 0.75rem; }
</style>
