<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { getHadiths, getHadithSharhPages, getCollections } from '$lib/api';
  import type { ApiHadith, ApiCollection, PaginatedResponse, SharhPageRef } from '$lib/types';
  import HadithCard from '$lib/components/hadith/HadithCard.svelte';
  import BookViewerModal from '$lib/components/reader/BookViewerModal.svelte';
  import Pagination from '$lib/components/common/Pagination.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let result: PaginatedResponse<ApiHadith> | null = $state(null);
  let loading = $state(true);
  let sharhMappings: Record<string, SharhPageRef> = $state({});
  let sharhTarget: { bookId: number; pageIndex: number; bookName: string; hadithNumber: number } | null = $state(null);
  let collections: ApiCollection[] = $state([]);

  let currentPage = $derived(Number(page.url.searchParams.get('page')) || 1);
  let bookFilter = $derived(page.url.searchParams.get('book') ? Number(page.url.searchParams.get('book')) : undefined);
  let activeCollectionName = $derived(
    bookFilter == null
      ? null
      : (collections.find(c => c.collection_id === bookFilter)?.name_en ?? `Book ${bookFilter}`)
  );

  async function load() {
    loading = true;
    try {
      result = await getHadiths({ book: bookFilter, page: currentPage });

      // Fetch sharh mappings for visible hadiths
      if (result && result.data.length > 0) {
        const numbers = result.data.map(h => h.hadith_number);
        const bookId = result.data[0]?.collection_id ?? 1;
        getHadithSharhPages(bookId, numbers)
          .then(res => { sharhMappings = res.mappings; })
          .catch(() => {});
      }
    } catch (e) {
      console.error('Failed to load hadiths:', e);
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    try { collections = await getCollections(); } catch (e) { console.error(e); }
  });

  $effect(() => {
    void currentPage;
    void bookFilter;
    load();
  });

  function changePage(newPage: number) {
    const sp = new URLSearchParams();
    sp.set('page', String(newPage));
    if (bookFilter) sp.set('book', String(bookFilter));
    window.history.pushState({}, '', `/hadiths?${sp}`);
  }
</script>

<div class="hadith-list">
  <aside class="book-filter-panel">
    <h2>Books</h2>
    <a class="book-card" class:active={bookFilter === undefined} href="/hadiths">
      <span class="card-label">All Books</span>
    </a>
    {#each collections as c (c.id)}
      <a
        class="book-card"
        class:active={bookFilter === c.collection_id}
        href={`/hadiths?book=${c.collection_id}`}
      >
        <span class="book-num">{c.collection_id}</span>
        <span class="book-title arabic" dir="rtl">{c.name_ar ?? c.name_en}</span>
        <span class="book-en">{c.name_en}</span>
      </a>
    {/each}
  </aside>

  <main class="list-main">
    <div class="list-header">
      <h1>Hadiths</h1>
      {#if activeCollectionName}
        <span class="filter-badge">{activeCollectionName}</span>
      {/if}
    </div>

    {#if loading}
      <LoadingSpinner />
    {:else if result && result.data.length > 0}
      <div class="list">
        {#each result.data as hadith (hadith.id)}
          <HadithCard
            {hadith}
            sharhPage={sharhMappings[String(hadith.hadith_number)]}
            onopensharh={(info) => { sharhTarget = info; }}
          />
        {/each}
      </div>
      <Pagination page={result.page} hasMore={result.has_more} onPageChange={changePage} />
    {:else}
      <div class="empty">No hadiths found.</div>
    {/if}
  </main>
</div>

{#if sharhTarget}
  <BookViewerModal
    bookId={sharhTarget.bookId}
    pageIndex={sharhTarget.pageIndex}
    title={sharhTarget.bookName}
    subtitle="Hadith {sharhTarget.hadithNumber}"
    onclose={() => { sharhTarget = null; }}
  />
{/if}

<style>
  .hadith-list {
    padding: 24px;
    max-width: 1200px;
    display: flex;
    gap: 24px;
    align-items: flex-start;
  }

  /* Sticky left sidepanel: book filter cards */
  .book-filter-panel {
    flex: 0 0 240px;
    position: sticky;
    top: 16px;
  }
  .book-filter-panel h2 {
    font-size: 1.05rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 12px;
  }
  .book-card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    margin-bottom: 8px;
    text-decoration: none;
    color: var(--text-primary);
    background: var(--bg-surface);
    transition: all var(--transition);
  }
  .book-card:hover {
    border-color: var(--accent);
  }
  .book-card.active {
    background: var(--accent-muted);
    border-color: var(--accent);
    color: var(--accent);
  }
  .card-label {
    font-weight: 600;
    font-size: 0.95rem;
  }
  .book-num {
    font-family: var(--font-mono);
    font-size: 0.7rem;
    color: var(--text-muted);
  }
  .book-title {
    font-size: 0.95rem;
    font-weight: 600;
    line-height: 1.5;
  }
  .book-en {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Main list */
  .list-main {
    flex: 1 1 auto;
    min-width: 0;
  }
  .list-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 20px;
  }
  .filter-badge {
    padding: 4px 12px;
    background: var(--accent-muted);
    color: var(--accent);
    border-radius: 20px;
    font-size: 0.8rem;
    font-weight: 500;
  }
  .list { display: flex; flex-direction: column; gap: 12px; }
  .empty { text-align: center; color: var(--text-muted); padding: 40px; }

  @media (max-width: 900px) {
    .hadith-list { flex-direction: column; }
    .book-filter-panel { flex: 1 1 auto; position: static; }
  }
</style>
