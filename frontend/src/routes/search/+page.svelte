<script lang="ts">
  import { page } from '$app/state';
  import { searchAll } from '$lib/api';
  import type { SearchResponse } from '$lib/types';
  import { formatScore } from '$lib/utils';
  import { language } from '$lib/stores/language';
  import { appConfig } from '$lib/stores/config';
  import { proseArabicFontSize } from '$lib/stores/preferences';
  import Badge from '$lib/components/common/Badge.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import HadithBody from '$lib/components/hadith/HadithBody.svelte';

  let result: SearchResponse | null = $state(null);
  let loading = $state(false);
  let query = $state('');
  let searchType: 'text' | 'semantic' | 'hybrid' = $state('text');
  let rerank = $state(false);

  // React to URL param changes (e.g., from TopBar navigation)
  let urlQuery = $derived(page.url.searchParams.get('q') || '');
  let urlType = $derived((page.url.searchParams.get('type') as 'text' | 'semantic' | 'hybrid') || 'text');
  let urlRerank = $derived(page.url.searchParams.get('rerank') === 'true');

  $effect(() => {
    if (urlQuery) {
      query = urlQuery;
      searchType = urlType;
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
      result = await searchAll(query, searchType, 20, rerank);
    } catch (e) {
      console.error('Search failed:', e);
    } finally {
      loading = false;
    }
  }

  function handleSubmit(e: Event) {
    e.preventDefault();
    const sp = new URLSearchParams();
    sp.set('q', query);
    sp.set('type', searchType);
    if (rerank && searchType === 'hybrid') sp.set('rerank', 'true');
    window.history.pushState({}, '', `/search?${sp}`);
    doSearch();
  }
</script>

<div class="search-page">
  <header class="page-header">
    <Eyebrow>Search</Eyebrow>
    <h1>Search the Library</h1>
  </header>

  <form class="search-form" onsubmit={handleSubmit}>
    <input type="text" placeholder="Search hadiths and narrators…" bind:value={query} class="search-input" />
    <div class="type-toggle">
      <button type="button" class="btn btn-soft btn-sm toggle-btn" class:active={searchType === 'text'} onclick={() => searchType = 'text'}>Text</button>
      {#if $appConfig.advanced_enabled}
        <button type="button" class="btn btn-soft btn-sm toggle-btn" class:active={searchType === 'semantic'} onclick={() => searchType = 'semantic'}>Semantic</button>
        <button type="button" class="btn btn-soft btn-sm toggle-btn" class:active={searchType === 'hybrid'} onclick={() => searchType = 'hybrid'}>Hybrid</button>
      {/if}
    </div>
    {#if $appConfig.advanced_enabled && searchType === 'hybrid'}
      <label class="rerank-toggle" title="Slower but better ranking for theological queries (~200ms).">
        <input type="checkbox" bind:checked={rerank} />
        <span>⚡ Precision mode</span>
      </label>
    {/if}
    <button type="submit" class="btn btn-primary btn-md">Search</button>
  </form>

  {#if loading}
    <LoadingSpinner />
  {:else if result}
    {#if result.hadiths.length > 0}
      <section class="results-section">
        <Eyebrow>Hadiths · {result.hadiths.length}</Eyebrow>
        <div class="results-list">
          {#each result.hadiths as h}
            <a href="/hadiths/{h.id}" class="result-row">
              <div class="result-meta">
                <Badge text="Book {h.collection_id}" />
                <span class="hadith-num mono">#{h.hadith_number}</span>
                {#if h.score}<span class="score mono">{formatScore(h.score)}</span>{/if}
              </div>
              {#if h.narrator_text}<p class="narrator">{h.narrator_text}</p>{/if}
              <HadithBody
                textAr={h.text_ar}
                textEn={h.text_en}
                language={$language}
                arabicSize={Math.min(1.2, $proseArabicFontSize)}
                englishSize={1}
                preview
                previewLength={200}
              />
            </a>
          {/each}
        </div>
      </section>
    {/if}

    {#if result.narrators.length > 0}
      <section class="results-section">
        <Eyebrow>Narrators · {result.narrators.length}</Eyebrow>
        <div class="results-list">
          {#each result.narrators as n}
            <a href="/narrators/{n.id}" class="result-row">
              <div class="result-meta">
                <span class="narrator-name">{n.name_ar || n.name_en}</span>
                {#if n.generation}<Badge text={n.generation} variant="accent" />{/if}
              </div>
              {#if n.name_ar}<p class="name-ar arabic-prose" dir="rtl">{n.name_ar}</p>{/if}
              {#if n.hadith_count}<span class="hadith-count mono">{n.hadith_count} hadiths</span>{/if}
            </a>
          {/each}
        </div>
      </section>
    {/if}

    {#if result.hadiths.length === 0 && result.narrators.length === 0}
      <div class="empty">No results found for "{result.query}".</div>
    {/if}
  {/if}
</div>

<style>
  .search-page {
    padding: var(--space-8) var(--space-6);
    max-width: var(--page-width);
    margin: 0 auto;
  }
  .page-header { margin-bottom: var(--space-6); }
  .page-header h1 {
    font-family: var(--font-serif);
    font-size: 2.1rem;
    margin: var(--space-2) 0 0;
    letter-spacing: var(--tracking-tight);
  }
  .search-form {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-6);
    align-items: center;
    flex-wrap: wrap;
  }
  .search-input { flex: 1; min-width: 200px; }
  .type-toggle { display: flex; gap: 0; }
  .toggle-btn { border-radius: 0; }
  .toggle-btn:first-child { border-top-left-radius: var(--radius); border-bottom-left-radius: var(--radius); }
  .toggle-btn:last-child  { border-top-right-radius: var(--radius); border-bottom-right-radius: var(--radius); }
  .toggle-btn:not(:first-child) { border-left: none; }
  .toggle-btn.active {
    background: var(--accent-muted);
    color: var(--accent);
    border-color: var(--accent);
  }
  .rerank-toggle {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-secondary);
    cursor: pointer;
    user-select: none;
  }
  .rerank-toggle input { margin: 0; cursor: pointer; }

  .results-section { margin-bottom: var(--space-8); }
  .results-section :global(.eyebrow) { margin-bottom: var(--space-3); display: inline-block; }
  .results-list { display: flex; flex-direction: column; gap: 0; }
  .result-row {
    display: block;
    padding: var(--space-4) 0;
    border-bottom: 1px solid var(--border-subtle);
    color: inherit;
    text-decoration: none;
    transition: background var(--transition);
  }
  .result-row:hover { background: var(--bg-hover); }
  .result-row:last-child { border-bottom: none; }
  .result-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }
  .hadith-num { color: var(--text-muted); font-size: var(--text-meta); }
  .score { margin-left: auto; color: var(--success); font-size: var(--text-meta); }
  .narrator {
    font-family: var(--font-serif);
    color: var(--text-secondary);
    font-size: var(--text-meta);
    font-style: italic;
    margin: 0 0 var(--space-2);
  }
  .narrator-name {
    font-family: var(--font-serif);
    font-weight: var(--font-weight-semibold);
    font-size: var(--text-body);
  }
  .name-ar { color: var(--text-secondary); font-size: var(--text-body); margin: var(--space-1) 0; }
  .hadith-count { color: var(--text-muted); font-size: var(--text-meta); }
  .empty { text-align: center; color: var(--text-muted); padding: var(--space-10); }
</style>
