<script lang="ts">
  import { searchAll, getHadiths, getMatnDiff } from '$lib/api';
  import type { ApiHadith, ApiHadithSearchResult, ApiMatnDiff } from '$lib/types';
  import DiffViewer from '$lib/components/hadith/DiffViewer.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import PageHeader from '$lib/components/common/PageHeader.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import Divider from '$lib/components/common/Divider.svelte';
  import SectionHeading from '$lib/components/common/SectionHeading.svelte';
  import Button from '$lib/components/common/Button.svelte';

  // Book names for filter dropdown
  const BOOKS = [
    { id: 0, label: 'All Books' },
    { id: 1, label: 'Sahih al-Bukhari' },
    { id: 2, label: 'Sahih Muslim' },
    { id: 3, label: 'Sunan Abu Dawud' },
    { id: 4, label: 'Jami al-Tirmidhi' },
    { id: 5, label: 'Sunan al-Nasai' },
    { id: 6, label: 'Sunan Ibn Majah' },
  ];

  // Side A state
  let queryA = $state('');
  let bookA = $state(0);
  let resultsA: ApiHadithSearchResult[] = $state([]);
  let searchingA = $state(false);
  let selectedA: ApiHadithSearchResult | null = $state(null);
  let debounceA: ReturnType<typeof setTimeout> | null = null;

  // Side B state
  let queryB = $state('');
  let bookB = $state(0);
  let resultsB: ApiHadithSearchResult[] = $state([]);
  let searchingB = $state(false);
  let selectedB: ApiHadithSearchResult | null = $state(null);
  let debounceB: ReturnType<typeof setTimeout> | null = null;

  // Diff state
  let diffResult: ApiMatnDiff | null = $state(null);
  let diffLoading = $state(false);

  async function doSearch(side: 'A' | 'B') {
    const query = side === 'A' ? queryA : queryB;
    const book = side === 'A' ? bookA : bookB;
    if (!query.trim()) {
      if (side === 'A') resultsA = [];
      else resultsB = [];
      return;
    }

    if (side === 'A') searchingA = true;
    else searchingB = true;

    try {
      const num = parseInt(query.trim());
      let hadiths: ApiHadithSearchResult[];

      if (!isNaN(num) && num > 0) {
        // Numeric query — search by hadith number (optionally within a book)
        const res = await getHadiths({ number: num, book: book > 0 ? book : undefined, limit: 20 });
        hadiths = res.data.map(h => ({
          id: h.id,
          hadith_number: h.hadith_number,
          collection_id: h.collection_id,
          text_ar: h.text_ar,
          text_en: h.text_en,
          narrator_text: h.narrator_text,
          score: null,
        }));
      } else {
        // Text query — full-text search
        const res = await searchAll(query, 'text', 15);
        hadiths = res.hadiths;
        if (book > 0) {
          hadiths = hadiths.filter(h => h.collection_id === book);
        }
      }

      if (side === 'A') resultsA = hadiths;
      else resultsB = hadiths;
    } catch (e) {
      console.error('Search failed:', e);
    } finally {
      if (side === 'A') searchingA = false;
      else searchingB = false;
    }
  }

  function onInputA() {
    if (debounceA) clearTimeout(debounceA);
    debounceA = setTimeout(() => doSearch('A'), 300);
  }

  function onInputB() {
    if (debounceB) clearTimeout(debounceB);
    debounceB = setTimeout(() => doSearch('B'), 300);
  }

  function selectA(h: ApiHadithSearchResult) {
    selectedA = h;
    resultsA = [];
    queryA = '';
    diffResult = null;
  }

  function selectB(h: ApiHadithSearchResult) {
    selectedB = h;
    resultsB = [];
    queryB = '';
    diffResult = null;
  }

  async function runDiff() {
    if (!selectedA || !selectedB || selectedA.id === selectedB.id) return;
    diffLoading = true;
    diffResult = null;
    try {
      diffResult = await getMatnDiff(selectedA.id, selectedB.id);
    } catch (e) {
      console.error('Diff failed:', e);
    } finally {
      diffLoading = false;
    }
  }

  function bookName(bookId: number): string {
    return BOOKS.find(b => b.id === bookId)?.label ?? `Book ${bookId}`;
  }

  function truncate(text: string | null, len: number): string {
    if (!text) return '';
    return text.length > len ? text.slice(0, len) + '...' : text;
  }
</script>

<svelte:head>
  <title>Diff Hadiths - Ilm</title>
</svelte:head>

<div class="page-shell">
  <PageHeader
    eyebrow="Compare"
    title="Matn Diff"
    subtitle="Compare the text of any two hadiths side by side. Search by text or hadith number."
  />

  <div class="selectors">
    <!-- Side A -->
    <article class="selector-panel">
      <div class="panel-eyebrow"><Eyebrow>Hadith A</Eyebrow></div>
      {#if selectedA}
        <div class="selected-card">
          <div class="selected-info">
            <span class="selected-ref">#{selectedA.hadith_number}</span>
            <span class="selected-book">{selectedA.text_ar ? bookName(selectedA.collection_id) : ''}</span>
          </div>
          <div class="selected-preview" dir="rtl">{truncate(selectedA.text_ar, 80)}</div>
          <button class="clear-btn" onclick={() => { selectedA = null; diffResult = null; }}>Change</button>
        </div>
      {:else}
        <div class="search-area">
          <div class="search-row">
            <input
              type="text"
              class="search-input"
              placeholder="Search by text or number..."
              bind:value={queryA}
              oninput={onInputA}
            />
            <select class="book-filter" bind:value={bookA} onchange={() => { if (queryA) doSearch('A'); }}>
              {#each BOOKS as b}
                <option value={b.id}>{b.label}</option>
              {/each}
            </select>
          </div>
          {#if searchingA}
            <div class="search-status">Searching...</div>
          {/if}
          {#if resultsA.length > 0}
            <div class="results-list">
              {#each resultsA as h}
                <button class="result-item" onclick={() => selectA(h)}>
                  <span class="result-ref">#{h.hadith_number} — {bookName(h.collection_id)}</span>
                  <span class="result-text" dir="rtl">{truncate(h.text_ar, 60)}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </article>

    <!-- Side B -->
    <article class="selector-panel">
      <div class="panel-eyebrow"><Eyebrow>Hadith B</Eyebrow></div>
      {#if selectedB}
        <div class="selected-card">
          <div class="selected-info">
            <span class="selected-ref">#{selectedB.hadith_number}</span>
            <span class="selected-book">{selectedB.text_ar ? bookName(selectedB.collection_id) : ''}</span>
          </div>
          <div class="selected-preview" dir="rtl">{truncate(selectedB.text_ar, 80)}</div>
          <button class="clear-btn" onclick={() => { selectedB = null; diffResult = null; }}>Change</button>
        </div>
      {:else}
        <div class="search-area">
          <div class="search-row">
            <input
              type="text"
              class="search-input"
              placeholder="Search by text or number..."
              bind:value={queryB}
              oninput={onInputB}
            />
            <select class="book-filter" bind:value={bookB} onchange={() => { if (queryB) doSearch('B'); }}>
              {#each BOOKS as b}
                <option value={b.id}>{b.label}</option>
              {/each}
            </select>
          </div>
          {#if searchingB}
            <div class="search-status">Searching...</div>
          {/if}
          {#if resultsB.length > 0}
            <div class="results-list">
              {#each resultsB as h}
                <button class="result-item" onclick={() => selectB(h)}>
                  <span class="result-ref">#{h.hadith_number} — {bookName(h.collection_id)}</span>
                  <span class="result-text" dir="rtl">{truncate(h.text_ar, 60)}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </article>
  </div>

  <div class="compare-action">
    <Button
      variant="primary"
      size="md"
      onclick={runDiff}
      disabled={!selectedA || !selectedB || selectedA.id === selectedB.id || diffLoading}
    >
      {diffLoading ? 'Computing…' : 'Compare'}
    </Button>
    {#if selectedA && selectedB && selectedA.id === selectedB.id}
      <span class="compare-warn">Select two different hadiths</span>
    {/if}
  </div>

  {#if diffLoading}
    <LoadingSpinner />
  {/if}

  {#if diffResult}
    <Divider variant="ornamental" />
    <SectionHeading eyebrow="Result" title="Diff" level={2} />
    <DiffViewer result={diffResult} />
  {/if}
</div>

<style>
  .selectors {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-4);
    margin-bottom: var(--space-5);
  }
  .selector-panel {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    padding: var(--space-4);
    min-height: 160px;
  }
  .panel-eyebrow { margin-bottom: var(--space-3); }

  .search-row {
    display: flex;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }
  .search-input {
    flex: 1;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    outline: none;
  }
  .search-input:focus { border-color: var(--accent); }
  .book-filter {
    padding: var(--space-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    min-width: 130px;
  }
  .search-status {
    font-family: var(--font-serif);
    font-style: italic;
    font-size: var(--text-meta);
    color: var(--text-muted);
    padding: var(--space-1) 0;
  }

  .results-list {
    max-height: 240px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }
  .result-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-2) var(--space-3);
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    text-align: left;
    cursor: pointer;
    transition: background var(--transition);
    width: 100%;
  }
  .result-item:hover { background: var(--bg-hover); }
  .result-ref {
    font-family: var(--font-mono);
    font-size: var(--text-meta);
    color: var(--accent);
    font-weight: var(--font-weight-semibold);
  }
  .result-text {
    font-size: var(--text-meta);
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Selected hadith card */
  .selected-card {
    background: var(--accent-muted);
    border: 1px solid var(--accent);
    border-radius: var(--radius);
    padding: var(--space-3);
  }
  .selected-info {
    display: flex;
    gap: var(--space-2);
    align-items: baseline;
    margin-bottom: var(--space-2);
  }
  .selected-ref {
    font-family: var(--font-mono);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-semibold);
    color: var(--accent);
  }
  .selected-book {
    font-family: var(--font-serif);
    font-size: var(--text-meta);
    color: var(--text-secondary);
    font-style: italic;
  }
  .selected-preview {
    font-size: var(--text-meta);
    color: var(--text-primary);
    line-height: 1.8;
    margin-bottom: var(--space-2);
    max-height: 60px;
    overflow: hidden;
  }
  .clear-btn {
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    text-decoration: underline;
    padding: 0;
  }
  .clear-btn:hover { color: var(--accent); }

  .compare-action {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-6);
  }
  .compare-warn {
    font-family: var(--font-serif);
    font-style: italic;
    font-size: var(--text-meta);
    color: var(--warning);
  }

  @media (max-width: 1024px) {
    .selectors { grid-template-columns: 1fr; }
  }
  @media (max-width: 768px) {
    .search-row { flex-direction: column; }
    .book-filter { min-width: unset; }
  }
</style>
