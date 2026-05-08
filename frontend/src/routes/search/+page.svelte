<script lang="ts">
  import { page } from '$app/state';
  import { searchAll } from '$lib/api';
  import type { SearchResponse } from '$lib/types';
  import { truncate, stripHtml, formatScore } from '$lib/utils';
  import { language } from '$lib/stores/language';
  import { appConfig } from '$lib/stores/config';
  import Badge from '$lib/components/common/Badge.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

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
  <h1>Search</h1>

  <form class="search-form" onsubmit={handleSubmit}>
    <input type="text" placeholder="Search hadiths and narrators..." bind:value={query} class="search-input" />
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
        <h2>Hadiths ({result.hadiths.length})</h2>
        <div class="results-list">
          {#each result.hadiths as h}
            <a href="/hadiths/{h.id}" class="result-card">
              <div class="result-header">
                <Badge text="Book {h.collection_id}" />
                <span class="hadith-num mono">#{h.hadith_number}</span>
                {#if h.score}<span class="score mono">{formatScore(h.score)}</span>{/if}
              </div>
              {#if h.narrator_text}<p class="narrator">{h.narrator_text}</p>{/if}
              <p class="text">{$language === 'en' && h.text_en ? truncate(stripHtml(h.text_en), 200) : truncate(h.text_ar || stripHtml(h.text_en), 200)}</p>
            </a>
          {/each}
        </div>
      </section>
    {/if}

    {#if result.narrators.length > 0}
      <section class="results-section">
        <h2>Narrators ({result.narrators.length})</h2>
        <div class="results-list">
          {#each result.narrators as n}
            <a href="/narrators/{n.id}" class="result-card">
              <div class="result-header">
                <span class="narrator-name">{n.name_ar || n.name_en}</span>
                {#if n.generation}<Badge text={n.generation} variant="accent" />{/if}
              </div>
              {#if n.name_ar}<p class="name-ar arabic" dir="rtl">{n.name_ar}</p>{/if}
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
  .search-page { padding: var(--space-6); }
  h1 { margin-bottom: var(--space-5); }
  .search-form { display: flex; gap: var(--space-2); margin-bottom: var(--space-6); align-items: center; flex-wrap: wrap; }
  .search-input { flex: 1; max-width: 100%; min-width: 200px; }
  .type-toggle { display: flex; gap: 0; }
  .toggle-btn { border-radius: 0; }
  .toggle-btn:first-child { border-top-left-radius: var(--radius); border-bottom-left-radius: var(--radius); }
  .toggle-btn:last-child { border-top-right-radius: var(--radius); border-bottom-right-radius: var(--radius); }
  .toggle-btn:not(:first-child) { border-left: none; }
  .toggle-btn.active { background: var(--accent); color: var(--btn-primary-fg); border-color: var(--accent); }
  .rerank-toggle { display: inline-flex; align-items: center; gap: var(--space-1); font-size: var(--text-sm); color: var(--text-secondary); cursor: pointer; user-select: none; }
  .rerank-toggle input { margin: 0; cursor: pointer; }
  .results-section { margin-bottom: var(--space-8); }
  .results-section h2 { margin-bottom: var(--space-3); }
  .results-list { display: flex; flex-direction: column; gap: var(--space-3); }
  .result-card { display: block; padding: var(--space-3) var(--space-4); background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); color: var(--text-primary); transition: all var(--transition); }
  .result-card:hover { border-color: var(--accent); background: var(--bg-hover); color: var(--text-primary); }
  .result-header { display: flex; align-items: center; gap: var(--space-3); margin-bottom: 6px; }
  .hadith-num { color: var(--text-muted); font-size: var(--text-sm); }
  .score { margin-left: auto; color: var(--success); font-size: var(--text-sm); }
  .narrator { color: var(--accent); font-size: var(--text-sm); margin-bottom: var(--space-1); }
  .text { color: var(--text-secondary); font-size: var(--text-sm); line-height: 1.5; }
  .narrator-name { font-weight: var(--font-weight-semibold); font-size: var(--text-base); }
  .name-ar { color: var(--text-secondary); font-size: var(--text-base); }
  .hadith-count { color: var(--text-muted); font-size: var(--text-sm); }
  .empty { text-align: center; color: var(--text-muted); padding: var(--space-10); }
</style>
