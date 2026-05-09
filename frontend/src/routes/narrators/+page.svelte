<script lang="ts">
  import { page } from '$app/state';
  import { getNarrators } from '$lib/api';
  import type { ApiNarratorWithCount, PaginatedResponse } from '$lib/types';
  import NarratorCard from '$lib/components/narrator/NarratorCard.svelte';
  import Pagination from '$lib/components/common/Pagination.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import Button from '$lib/components/common/Button.svelte';

  let result: PaginatedResponse<ApiNarratorWithCount> | null = $state(null);
  let loading = $state(true);
  let searchQuery = $state('');
  let selectedGeneration = $state('');

  const generations = [
    { value: '', label: 'All' },
    { value: '1', label: '1 - Sahaba' },
    { value: '2', label: '2 - Tabi\'in' },
    { value: '3', label: '3 - Tabi\' al-Tabi\'in' },
    { value: '4', label: '4th' },
    { value: '5', label: '5th' },
    { value: '6', label: '6th' },
    { value: '7', label: '7th' },
    { value: '8', label: '8th' },
    { value: '9', label: '9th' },
    { value: '10', label: '10th' },
    { value: '11', label: '11th' },
    { value: '12', label: '12th' },
  ];

  let currentPage = $derived(Number(page.url.searchParams.get('page')) || 1);

  async function load() {
    loading = true;
    try {
      result = await getNarrators({
        q: searchQuery || undefined,
        page: currentPage,
        generation: selectedGeneration || undefined,
      });
    } catch (e) {
      console.error('Failed to load narrators:', e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void currentPage;
    load();
  });

  function handleSearch(e: Event) {
    e.preventDefault();
    load();
  }

  function selectGeneration(gen: string) {
    selectedGeneration = gen;
    changePage(1);
    load();
  }

  function changePage(newPage: number) {
    const sp = new URLSearchParams();
    sp.set('page', String(newPage));
    if (searchQuery) sp.set('q', searchQuery);
    if (selectedGeneration) sp.set('generation', selectedGeneration);
    window.history.pushState({}, '', `/narrators?${sp}`);
  }
</script>

<div class="narrator-list">
  <header class="page-header">
    <Eyebrow>Narrators</Eyebrow>
    <h1>Companions and Transmitters</h1>
    <p class="subtitle">Search narrators across generations of the chain.</p>
  </header>

  <form class="search-bar" onsubmit={handleSearch}>
    <input type="text" placeholder="Search narrators (Arabic or Latin)…" bind:value={searchQuery} />
    <Button type="submit" variant="primary" size="md">Search</Button>
  </form>

  <div class="generation-eyebrow"><Eyebrow tone="muted">Generation</Eyebrow></div>
  <div class="generation-tabs">
    {#each generations as gen}
      <button
        class="gen-tab"
        class:active={selectedGeneration === gen.value}
        onclick={() => selectGeneration(gen.value)}
      >
        {gen.label}
      </button>
    {/each}
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else if result && result.data.length > 0}
    <div class="grid">
      {#each result.data as narrator (narrator.id)}
        <NarratorCard {narrator} />
      {/each}
    </div>
    <Pagination page={result.page} hasMore={result.has_more} onPageChange={changePage} />
  {:else}
    <div class="empty">No narrators found.</div>
  {/if}
</div>

<style>
  .narrator-list {
    padding: var(--space-8) var(--space-6);
    max-width: var(--page-width);
    margin: 0 auto;
  }

  .page-header { margin-bottom: var(--space-6); }
  .page-header h1 {
    font-family: var(--font-serif);
    font-size: 2.1rem;
    margin: var(--space-2) 0;
    letter-spacing: var(--tracking-tight);
  }
  .page-header .subtitle {
    margin: 0;
    font-family: var(--font-serif);
    font-style: italic;
    color: var(--text-secondary);
    font-size: var(--text-body);
  }

  .search-bar {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-5);
  }
  .search-bar input { flex: 1; max-width: 480px; }

  .generation-eyebrow { margin-bottom: var(--space-2); }
  .generation-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-bottom: var(--space-6);
  }
  .gen-tab {
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    font-weight: var(--font-weight-medium);
    padding: var(--space-1) var(--space-3);
    border-radius: var(--radius-pill);
    color: var(--text-secondary);
    background: transparent;
    border: 1px solid var(--border);
    cursor: pointer;
    transition: all var(--transition);
  }
  .gen-tab:hover {
    background: var(--bg-hover);
    border-color: var(--btn-border-hover);
  }
  .gen-tab.active {
    background: var(--accent-muted);
    color: var(--accent);
    border-color: var(--accent);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: var(--space-4);
  }
  .empty { text-align: center; color: var(--text-muted); padding: var(--space-10); }
</style>
