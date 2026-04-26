<script lang="ts">
  import NarratorAutocomplete from '$lib/components/isnad/NarratorAutocomplete.svelte';
  import IsnadChip from '$lib/components/isnad/IsnadChip.svelte';
  import IsnadResults from '$lib/components/isnad/IsnadResults.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import { isnadSearch } from '$lib/api';
  import type { ApiNarratorSearchResult, IsnadSearchResponse } from '$lib/types';

  let selectedNarrators: ApiNarratorSearchResult[] = $state([]);
  let mode: 'loose' | 'strict' = $state('loose');
  let result: IsnadSearchResponse | null = $state(null);
  let loading = $state(false);

  let excludeIds = $derived(selectedNarrators.map(n => n.id));

  function addNarrator(n: ApiNarratorSearchResult) {
    selectedNarrators = [...selectedNarrators, n];
  }

  function removeNarrator(id: string) {
    selectedNarrators = selectedNarrators.filter(n => n.id !== id);
    result = null;
  }

  function clearAll() {
    selectedNarrators = [];
    result = null;
  }

  async function doSearch() {
    if (selectedNarrators.length < 2) return;
    loading = true;
    try {
      result = await isnadSearch({
        narrator_ids: selectedNarrators.map(n => n.id),
        mode,
        limit: 20,
      });
    } catch (e) {
      console.error('Isnad search failed:', e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="isnad-page">
  <h1>Isnad Search</h1>
  <p class="subtitle">Find hadiths by selecting narrators in the chain</p>

  <div class="mode-toggle">
    <button class="toggle-btn" class:active={mode === 'loose'} onclick={() => { mode = 'loose'; result = null; }}>Any Order</button>
    <button class="toggle-btn" class:active={mode === 'strict'} onclick={() => { mode = 'strict'; result = null; }}>Ordered Chain</button>
  </div>

  <div class="chain-builder">
    {#if selectedNarrators.length > 0}
      <div class="selected-chain" class:strict={mode === 'strict'}>
        {#each selectedNarrators as n, i (n.id)}
          {#if mode === 'strict' && i > 0}
            <span class="chain-arrow">&rarr;</span>
          {/if}
          <IsnadChip narrator={n} onRemove={() => removeNarrator(n.id)} />
        {/each}
      </div>
    {/if}

    <NarratorAutocomplete onSelect={addNarrator} {excludeIds} placeholder="Search narrator to add..." />
  </div>

  <div class="actions">
    <button class="search-btn" disabled={selectedNarrators.length < 2 || loading} onclick={doSearch}>
      Search ({selectedNarrators.length} narrators)
    </button>
    {#if selectedNarrators.length > 0}
      <button class="clear-btn" onclick={clearAll}>Clear All</button>
    {/if}
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if result}
    <IsnadResults {result} {selectedNarrators} />
  {/if}
</div>

<style>
  .isnad-page { padding: 24px; }
  h1 { margin-bottom: 4px; }
  .subtitle { color: var(--text-secondary); font-size: 0.85rem; margin-bottom: 20px; }
  .mode-toggle { display: flex; border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; margin-bottom: 16px; width: fit-content; }
  .toggle-btn { padding: 8px 18px; font-size: 0.8rem; font-weight: 500; background: var(--bg-surface); color: var(--text-secondary); border: none; cursor: pointer; transition: all var(--transition); }
  .toggle-btn.active { background: var(--accent); color: var(--bg-primary); }
  .toggle-btn:hover:not(.active) { background: var(--bg-hover); }
  .chain-builder { margin-bottom: 16px; }
  .selected-chain { display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-bottom: 12px; }
  .chain-arrow { color: var(--text-muted); font-size: 1rem; }
  .actions { display: flex; gap: 8px; margin-bottom: 24px; }
  .search-btn { padding: 8px 20px; background: var(--accent); color: var(--bg-primary); border-radius: var(--radius); font-weight: 600; font-size: 0.85rem; transition: background var(--transition); }
  .search-btn:hover:not(:disabled) { background: var(--accent-hover); }
  .search-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .clear-btn { padding: 8px 16px; background: var(--bg-surface); color: var(--text-secondary); border: 1px solid var(--border); border-radius: var(--radius); font-size: 0.85rem; cursor: pointer; transition: all var(--transition); }
  .clear-btn:hover { border-color: var(--accent); color: var(--accent); }
</style>
