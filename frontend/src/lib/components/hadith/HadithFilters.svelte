<script lang="ts">
  import type { ApiCollection, ApiNarratorSearchResult } from '$lib/types';
  import CollectionFilter from './CollectionFilter.svelte';
  import NumberRangeFilter from './NumberRangeFilter.svelte';
  import NarratorFilter from './NarratorFilter.svelte';

  let {
    collections,
    books,
    nMin,
    nMax,
    narrators,
    narratorDetails,
    onChange,
  }: {
    collections: ApiCollection[];
    books: number[];
    nMin: number | undefined;
    nMax: number | undefined;
    narrators: string[];
    narratorDetails: ApiNarratorSearchResult[];
    onChange: (patch: {
      books?: number[];
      nMin?: number | undefined;
      nMax?: number | undefined;
      narrators?: string[];
      narratorDetails?: ApiNarratorSearchResult[];
    }) => void;
  } = $props();

  const activeFilterCount = $derived(
    (books.length > 0 ? 1 : 0) +
      (nMin !== undefined || nMax !== undefined ? 1 : 0) +
      (narrators.length > 0 ? 1 : 0)
  );

  function clearAll() {
    onChange({
      books: [],
      nMin: undefined,
      nMax: undefined,
      narrators: [],
      narratorDetails: [],
    });
  }
</script>

<div class="filters">
  {#if activeFilterCount >= 2}
    <div class="aggregate-clear">
      <button type="button" class="clear-all" onclick={clearAll}>
        Clear all filters
      </button>
    </div>
  {/if}

  <CollectionFilter
    {collections}
    selected={books}
    onChange={(next) => onChange({ books: next })}
  />

  <NumberRangeFilter
    min={nMin}
    max={nMax}
    onChange={({ min, max }) => onChange({ nMin: min, nMax: max })}
  />

  <NarratorFilter
    selected={narrators}
    selectedDetails={narratorDetails}
    onChange={({ ids, details }) => onChange({ narrators: ids, narratorDetails: details })}
  />
</div>

<style>
  .filters {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .aggregate-clear {
    display: flex;
    justify-content: flex-end;
    margin-bottom: calc(-1 * var(--space-2));
  }
  .clear-all {
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--accent);
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .clear-all:hover { color: var(--accent-hover); }
</style>
