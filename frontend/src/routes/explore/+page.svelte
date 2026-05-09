<script lang="ts">
  import { page } from '$app/state';
  import { searchUnified } from '$lib/api';
  import type { UnifiedSearchResponse, UnifiedSearchItem } from '$lib/types';
  import { truncate, stripHtml } from '$lib/utils';
  import { language } from '$lib/stores/language';
  import AyahCard from '$lib/components/quran/AyahCard.svelte';
  import Badge from '$lib/components/common/Badge.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import Pagination from '$lib/components/common/Pagination.svelte';
  import PageHeader from '$lib/components/common/PageHeader.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import Button from '$lib/components/common/Button.svelte';
  import Ornament from '$lib/components/common/Ornament.svelte';
  import { appConfig } from '$lib/stores/config';

  let result: UnifiedSearchResponse | null = $state(null);
  let loading = $state(false);
  let query = $state('');
  let searchType: 'hybrid' | 'semantic' = $state('semantic');
  let currentPage = $state(1);
  let rerank = $state(false);

  let urlQuery = $derived(page.url.searchParams.get('q') || '');
  let urlType = $derived((page.url.searchParams.get('type') as 'hybrid' | 'semantic') || 'semantic');
  let urlPage = $derived(Number(page.url.searchParams.get('page')) || 1);
  let urlRerank = $derived(page.url.searchParams.get('rerank') === 'true');

  $effect(() => {
    if (urlQuery) {
      query = urlQuery;
      searchType = urlType;
      currentPage = urlPage;
      rerank = urlRerank;
      doSearch();
    }
  });

  // Clear rerank when switching away from hybrid — it only applies there
  $effect(() => {
    if (searchType !== 'hybrid' && rerank) rerank = false;
  });

  async function doSearch() {
    if (!query.trim()) return;
    loading = true;
    try {
      result = await searchUnified(query, searchType, 20, currentPage, rerank);
    } catch (e) {
      console.error('Unified search failed:', e);
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
  }

  function pushUrl() {
    const sp = new URLSearchParams();
    sp.set('q', query);
    sp.set('type', searchType);
    if (currentPage > 1) sp.set('page', String(currentPage));
    if (rerank && searchType === 'hybrid') sp.set('rerank', 'true');
    window.history.pushState({}, '', `/explore?${sp}`);
  }

  function isQuran(item: UnifiedSearchItem): item is Extract<UnifiedSearchItem, { source: 'quran' }> {
    return item.source === 'quran';
  }
</script>

{#if !$appConfig.advanced_enabled}
<div class="page-shell">
  <PageHeader eyebrow="Explore" title="Semantic Search" />
  <p class="unavailable-msg">Advanced search features are not available in this build. Use <a class="link" href="/search">Search</a> for text search.</p>
</div>
{:else}
<div class="page-shell">
  <PageHeader
    eyebrow="Explore"
    title="Semantic Search"
    subtitle="Find verses and hadiths by meaning, not just keywords."
  />

  <form class="explore-form" onsubmit={handleSubmit}>
    <div class="search-bar">
      <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
        <circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
      </svg>
      <input type="text" placeholder="Ask a question or search a topic…" bind:value={query} class="search-input" />
      <Button type="submit" variant="primary" size="md">Search</Button>
    </div>
    <div class="search-controls">
      <div class="type-toggle">
        <button type="button" class="toggle-btn" class:active={searchType === 'hybrid'} onclick={() => searchType = 'hybrid'}>Hybrid</button>
        <button type="button" class="toggle-btn" class:active={searchType === 'semantic'} onclick={() => searchType = 'semantic'}>Semantic</button>
      </div>
      {#if searchType === 'hybrid'}
        <label class="rerank-toggle" title="Slower but better ranking for theological queries (~200ms).">
          <input type="checkbox" bind:checked={rerank} />
          <span>⚡ Precision mode</span>
        </label>
      {/if}
    </div>
  </form>

  {#if loading}
    <LoadingSpinner />
  {:else if result}
    {#if result.results.length > 0}
      <div class="results-summary">
        <span class="summary-count">{result.results.length}</span>
        <span class="summary-label">results</span>
        <span class="summary-dot">·</span>
        <span class="quran-count">{result.quran_count} ayāt</span>
        <span class="summary-dot">·</span>
        <span class="hadith-count">{result.hadith_count} hadiths</span>
      </div>

      <div class="results-list">
        {#each result.results as item}
          {#if isQuran(item)}
            <article class="result-row source-quran">
              <div class="source-eyebrow"><Eyebrow>Qurʾān</Eyebrow></div>
              <a href="/quran/{item.surah_number}?ayah={item.ayah_number}" class="result-link">
                <AyahCard ayah={{
                  id: item.id,
                  surah_number: item.surah_number,
                  ayah_number: item.ayah_number,
                  text_ar: item.text_ar,
                  text_en: item.text_en,
                  tafsir_en: item.tafsir_en,
                }} compact />
              </a>
            </article>
          {:else}
            <article class="result-row source-hadith">
              <div class="source-eyebrow"><Eyebrow tone="muted">Hadith</Eyebrow></div>
              <a href="/hadiths/{item.id}" class="result-card">
                <div class="result-header">
                  <Badge text="Book {item.collection_id}" />
                  <span class="hadith-num mono">#{item.hadith_number}</span>
                  {#if item.score}<span class="score mono">{item.score.toFixed(3)}</span>{/if}
                </div>
                {#if item.narrator_text}<p class="narrator">{item.narrator_text}</p>{/if}
                <p class="text">{$language === 'en' && item.text_en ? truncate(stripHtml(item.text_en), 200) : truncate(item.text_ar || stripHtml(item.text_en ?? ''), 200)}</p>
              </a>
            </article>
          {/if}
        {/each}
      </div>

      <Pagination page={result.page} hasMore={result.has_more} onPageChange={changePage} />
    {:else}
      <div class="empty">No results found for "{result.query}".</div>
    {/if}
  {:else}
    <div class="empty-state">
      <Ornament variant="star" size={32} color="var(--accent)" />
      <h2 class="empty-title">Search across Qurʾān &amp; Sunnah</h2>
      <p class="empty-hint">Find wisdom from the Qurʾān and Prophetic tradition in a single search.</p>
    </div>
  {/if}
</div>
{/if}

<style>
  .unavailable-msg {
    color: var(--text-secondary);
    font-family: var(--font-serif);
    font-size: var(--text-body);
    font-style: italic;
  }
  .link {
    color: var(--accent);
    text-decoration: underline;
    text-underline-offset: 0.2em;
  }

  .explore-form {
    margin-bottom: var(--space-6);
  }
  .search-bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    max-width: 720px;
    padding: var(--space-2) var(--space-2) var(--space-2) var(--space-4);
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    transition: border-color var(--transition);
  }
  .search-bar:focus-within { border-color: var(--accent); }
  .search-icon {
    width: 18px;
    height: 18px;
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .search-input {
    flex: 1;
    border: none;
    background: transparent;
    outline: none;
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--text-body);
    padding: var(--space-2) 0;
  }
  .search-input::placeholder { color: var(--text-muted); }

  .search-controls {
    display: flex;
    gap: var(--space-3);
    align-items: center;
    margin-top: var(--space-3);
    flex-wrap: wrap;
  }
  .type-toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .toggle-btn {
    padding: var(--space-3) var(--space-4);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-medium);
    background: transparent;
    color: var(--text-secondary);
    transition: all var(--transition);
    border: none;
    cursor: pointer;
  }
  .toggle-btn.active {
    background: var(--accent-muted);
    color: var(--accent);
  }
  .rerank-toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-secondary);
    cursor: pointer;
    user-select: none;
  }
  .rerank-toggle input { margin: 0; cursor: pointer; }

  .results-summary {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    margin-bottom: var(--space-5);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-secondary);
  }
  .summary-count {
    font-family: var(--font-serif);
    font-size: var(--text-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
  }
  .summary-label { color: var(--text-muted); }
  .quran-count { color: var(--success); }
  .hadith-count { color: var(--accent); }
  .summary-dot { color: var(--text-muted); }

  .results-list {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .result-row {
    padding: var(--space-4) 0;
    border-bottom: 1px solid var(--border-subtle);
  }
  .result-row:last-child { border-bottom: none; }
  .source-eyebrow { margin-bottom: var(--space-2); }

  .result-link {
    display: block;
    color: var(--text-primary);
    text-decoration: none;
  }
  .result-link:hover { color: var(--text-primary); }

  .result-card {
    display: block;
    padding: var(--space-3) var(--space-4);
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    color: var(--text-primary);
    transition: all var(--transition);
  }
  .result-card:hover { border-color: var(--accent); background: var(--bg-hover); color: var(--text-primary); }
  .result-header { display: flex; align-items: center; gap: var(--space-3); margin-bottom: var(--space-2); }
  .hadith-num { color: var(--text-muted); font-size: var(--text-meta); }
  .score { margin-left: auto; color: var(--success); font-size: var(--text-meta); }
  .narrator {
    font-family: var(--font-serif);
    color: var(--text-secondary);
    font-style: italic;
    font-size: var(--text-meta);
    margin: 0 0 var(--space-2);
  }
  .text {
    font-family: var(--font-serif);
    color: var(--text-primary);
    font-size: var(--text-body);
    line-height: var(--leading-relaxed);
    margin: 0;
  }

  .empty {
    text-align: center;
    color: var(--text-muted);
    padding: var(--space-12);
    font-family: var(--font-serif);
    font-style: italic;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: var(--space-12) var(--space-6);
    color: var(--text-secondary);
    gap: var(--space-3);
  }
  .empty-title {
    color: var(--text-primary);
    font-family: var(--font-serif);
    font-size: var(--text-lead);
    font-weight: var(--font-weight-semibold);
    margin: var(--space-2) 0 0;
  }
  .empty-hint {
    max-width: 420px;
    line-height: 1.6;
    font-family: var(--font-serif);
    font-style: italic;
    font-size: var(--text-body);
    color: var(--text-muted);
    margin: 0;
  }

  @media (max-width: 640px) {
    .search-bar { max-width: 100%; }
  }
</style>
