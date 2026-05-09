<script lang="ts">
  import NarratorAutocomplete from '$lib/components/isnad/NarratorAutocomplete.svelte';
  import IsnadChip from '$lib/components/isnad/IsnadChip.svelte';
  import IsnadResults from '$lib/components/isnad/IsnadResults.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import PageHeader from '$lib/components/common/PageHeader.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
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

<div class="page-shell">
  <PageHeader
    eyebrow="Search"
    title="Isnād Search"
    subtitle="Find every hadith chain that contains the narrators you select."
  />

  <div class="mode-row">
    <div class="mode-eyebrow"><Eyebrow tone="muted">Match Mode</Eyebrow></div>
    <div class="mode-toggle">
      <button class="btn btn-soft btn-sm toggle-btn" class:active={mode === 'loose'} onclick={() => { mode = 'loose'; result = null; }}>Any Order</button>
      <button class="btn btn-soft btn-sm toggle-btn" class:active={mode === 'strict'} onclick={() => { mode = 'strict'; result = null; }}>Ordered Chain</button>
    </div>
  </div>

  <div class="chain-builder">
    <div class="builder-eyebrow"><Eyebrow>Chain</Eyebrow></div>
    {#if selectedNarrators.length > 0}
      <div class="selected-chain" class:strict={mode === 'strict'}>
        {#each selectedNarrators as n, i (n.id)}
          {#if mode === 'strict' && i > 0}
            <span class="chain-arrow">→</span>
          {/if}
          <IsnadChip narrator={n} onRemove={() => removeNarrator(n.id)} />
        {/each}
      </div>
    {/if}

    <NarratorAutocomplete onSelect={addNarrator} {excludeIds} placeholder="Search narrator to add…" />
  </div>

  <div class="actions">
    <button class="btn btn-primary btn-md" disabled={selectedNarrators.length < 2 || loading} onclick={doSearch}>
      Search ({selectedNarrators.length} narrator{selectedNarrators.length === 1 ? '' : 's'})
    </button>
    {#if selectedNarrators.length > 0}
      <button class="btn btn-soft btn-md" onclick={clearAll}>Clear All</button>
    {/if}
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if result}
    <IsnadResults {result} {selectedNarrators} />
  {/if}
</div>

<style>
  .mode-row { margin-bottom: var(--space-5); }
  .mode-eyebrow { margin-bottom: var(--space-2); }
  .mode-toggle {
    display: flex;
    gap: 0;
    width: fit-content;
  }
  .toggle-btn { border-radius: 0; }
  .toggle-btn:first-child {
    border-top-left-radius: var(--radius);
    border-bottom-left-radius: var(--radius);
  }
  .toggle-btn:last-child {
    border-top-right-radius: var(--radius);
    border-bottom-right-radius: var(--radius);
  }
  .toggle-btn:not(:first-child) { border-left: none; }
  .toggle-btn.active {
    background: var(--accent-muted);
    color: var(--accent);
    border-color: var(--accent);
  }

  .chain-builder { margin-bottom: var(--space-5); }
  .builder-eyebrow { margin-bottom: var(--space-2); }
  .selected-chain {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }
  .chain-arrow {
    color: var(--accent);
    font-size: var(--text-base);
    font-weight: var(--font-weight-semibold);
  }

  .actions {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-6);
  }
</style>
