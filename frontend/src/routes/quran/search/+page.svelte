<script lang="ts">
  import { page } from '$app/state';
  import { searchQuran, searchByRoot } from '$lib/api';
  import type { QuranSearchResponse, RootSearchResponse } from '$lib/types';
  import AyahCard from '$lib/components/quran/AyahCard.svelte';
  import Pagination from '$lib/components/common/Pagination.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import PageHeader from '$lib/components/common/PageHeader.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import Button from '$lib/components/common/Button.svelte';
  import { appConfig } from '$lib/stores/config';

  let result: QuranSearchResponse | null = $state(null);
  let rootResult: RootSearchResponse | null = $state(null);
  let loading = $state(false);
  let query = $state('');
  let searchType: 'text' | 'semantic' | 'hybrid' | 'root' = $state('text');
  let currentPage = $state(1);

  let urlQuery = $derived(page.url.searchParams.get('q') || '');
  let urlType = $derived((page.url.searchParams.get('type') as typeof searchType) || 'text');
  let urlPage = $derived(Number(page.url.searchParams.get('page')) || 1);

  $effect(() => {
    if (urlQuery) {
      query = urlQuery;
      searchType = urlType;
      currentPage = urlPage;
      doSearch();
    }
  });

  async function doSearch() {
    if (!query.trim()) return;
    loading = true;
    result = null;
    rootResult = null;
    try {
      if (searchType === 'root') {
        rootResult = await searchByRoot(query);
      } else {
        result = await searchQuran(query, searchType as 'text' | 'semantic' | 'hybrid', 20, currentPage);
      }
    } catch (e) {
      console.error('Quran search failed:', e);
    } finally {
      loading = false;
    }
  }

  function handleSubmit(e: Event) {
    e.preventDefault();
    currentPage = 1;
    pushUrl();
    doSearch();
  }

  function changePage(newPage: number) {
    currentPage = newPage;
    pushUrl();
    doSearch();
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  function pushUrl() {
    const sp = new URLSearchParams();
    sp.set('q', query);
    sp.set('type', searchType);
    if (currentPage > 1) sp.set('page', String(currentPage));
    window.history.pushState({}, '', `/quran/search?${sp}`);
  }
</script>

<div class="page-shell">
  <PageHeader
    eyebrow="Qurʾān"
    title="Search the Qurʾān"
    subtitle="Full-text, semantic, hybrid, or trilateral root search across all 6,236 āyāt."
  />

  <form class="search-form" onsubmit={handleSubmit}>
    <input type="text" placeholder="Search the Qurʾān…" bind:value={query} class="search-input" />
    <div class="type-toggle">
      <button type="button" class="toggle-btn" class:active={searchType === 'text'} onclick={() => searchType = 'text'}>Text</button>
      {#if $appConfig.advanced_enabled}
        <button type="button" class="toggle-btn" class:active={searchType === 'semantic'} onclick={() => searchType = 'semantic'}>Semantic</button>
        <button type="button" class="toggle-btn" class:active={searchType === 'hybrid'} onclick={() => searchType = 'hybrid'}>Hybrid</button>
      {/if}
      <button type="button" class="toggle-btn" class:active={searchType === 'root'} onclick={() => searchType = 'root'}>Root</button>
    </div>
    <Button type="submit" variant="primary" size="md">Search</Button>
  </form>

  {#if loading}
    <LoadingSpinner />
  {:else if rootResult}
    {#if rootResult.occurrences.length > 0}
      <section class="results-section">
        <Eyebrow>Root Results</Eyebrow>
        <h2 class="results-title arabic-prose" dir="rtl">{rootResult.root}</h2>
        <p class="results-subtitle">{rootResult.occurrences.length} words across {rootResult.ayah_count} āyāt</p>
        <a href="/quran/root/{encodeURIComponent(rootResult.root)}" class="link">View detailed root page →</a>
      </section>
    {:else}
      <div class="empty">No words found for root "{query}".</div>
    {/if}
  {:else if result}
    {#if result.ayahs.length > 0}
      <section class="results-section">
        <Eyebrow>Results · {result.ayahs.length}</Eyebrow>
        <div class="results-list">
          {#each result.ayahs as ayah}
            <a href="/quran/{ayah.surah_number}?ayah={ayah.ayah_number}" class="result-link">
              <AyahCard {ayah} showScore compact />
            </a>
          {/each}
        </div>
        <Pagination page={result.page} hasMore={result.has_more} onPageChange={changePage} />
      </section>
    {:else}
      <div class="empty">No results found for "{result.query}".</div>
    {/if}
  {/if}
</div>

<style>
  .search-form {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-6);
    align-items: center;
    flex-wrap: wrap;
  }
  .search-input {
    flex: 1;
    min-width: 250px;
    max-width: 500px;
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    outline: none;
  }
  .search-input:focus { border-color: var(--accent); }

  .type-toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .toggle-btn {
    padding: var(--space-2) var(--space-4);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-medium);
    background: transparent;
    color: var(--text-secondary);
    border: none;
    cursor: pointer;
    transition: all var(--transition);
  }
  .toggle-btn.active {
    background: var(--accent-muted);
    color: var(--accent);
  }

  .results-section { margin-bottom: var(--space-7); }
  .results-section :global(.eyebrow) {
    display: inline-block;
    margin-bottom: var(--space-3);
  }
  .results-title {
    font-size: clamp(2rem, 5vw, 2.6rem);
    font-weight: var(--font-weight-semibold);
    margin: 0;
    line-height: 1.4;
  }
  .results-subtitle {
    font-family: var(--font-serif);
    font-size: var(--text-meta);
    color: var(--text-secondary);
    font-style: italic;
    margin: var(--space-2) 0;
  }
  .results-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .result-link {
    color: var(--text-primary);
    text-decoration: none;
  }
  .result-link:hover { color: var(--text-primary); }

  .link {
    color: var(--accent);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    text-decoration: underline;
    text-underline-offset: 0.2em;
  }
  .empty {
    text-align: center;
    color: var(--text-muted);
    padding: var(--space-12);
    font-family: var(--font-serif);
    font-style: italic;
  }
</style>
