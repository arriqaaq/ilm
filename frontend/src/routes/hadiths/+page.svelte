<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { getHadiths, getHadithSharhPages, getCollections } from '$lib/api';
  import type { ApiHadith, ApiCollection, PaginatedResponse, SharhPageRef } from '$lib/types';
  import HadithCard from '$lib/components/hadith/HadithCard.svelte';
  import BookViewerModal from '$lib/components/reader/BookViewerModal.svelte';
  import Pagination from '$lib/components/common/Pagination.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';

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
    <div class="filter-eyebrow"><Eyebrow>Collections</Eyebrow></div>
    <a class="book-card" class:active={bookFilter === undefined} href="/hadiths">
      <span class="book-title">All Books</span>
    </a>
    {#each collections as c (c.id)}
      <a
        class="book-card"
        class:active={bookFilter === c.collection_id}
        href={`/hadiths?book=${c.collection_id}`}
      >
        <span class="book-num mono">{c.collection_id}</span>
        <span class="book-ar arabic-prose" dir="rtl">{c.name_ar ?? c.name_en}</span>
        <span class="book-en">{c.name_en}</span>
      </a>
    {/each}
  </aside>

  <main class="list-main">
    <header class="list-header">
      <Eyebrow>Ḥadīth</Eyebrow>
      <h1>Hadiths</h1>
      {#if activeCollectionName}
        <p class="subtitle">Filter: {activeCollectionName}</p>
      {/if}
    </header>

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
    padding: var(--space-8) var(--space-6);
    max-width: var(--page-width);
    margin: 0 auto;
    display: flex;
    gap: var(--space-8);
    align-items: flex-start;
  }

  .book-filter-panel {
    flex: 0 0 240px;
    position: sticky;
    top: var(--space-4);
  }
  .filter-eyebrow { padding: 0 var(--space-2) var(--space-3); }

  .book-card {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3);
    border-bottom: 1px solid var(--border-subtle);
    text-decoration: none;
    color: var(--text-primary);
    transition: background var(--transition);
  }
  .book-card:hover { background: var(--bg-hover); }
  .book-card.active {
    background: var(--accent-muted);
    border-bottom-color: var(--accent);
  }
  .book-num {
    font-size: var(--text-2xs);
    color: var(--text-muted);
  }
  .book-ar {
    font-size: 1.05rem;
    color: var(--text-primary);
    font-weight: var(--font-weight-semibold);
    line-height: 1.5;
  }
  .book-en {
    font-size: var(--text-meta);
    color: var(--text-secondary);
  }
  .book-title {
    font-family: var(--font-serif);
    font-size: var(--text-body);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
  }

  .list-main { flex: 1 1 auto; min-width: 0; }
  .list-header { margin-bottom: var(--space-6); }
  .list-header h1 {
    font-family: var(--font-serif);
    font-size: 2.1rem;
    margin: var(--space-2) 0;
    letter-spacing: var(--tracking-tight);
  }
  .list-header .subtitle {
    margin: 0;
    font-family: var(--font-serif);
    font-style: italic;
    color: var(--text-secondary);
    font-size: var(--text-body);
  }
  .list { display: flex; flex-direction: column; gap: 0; }
  .empty { text-align: center; color: var(--text-muted); padding: var(--space-10); }

  @media (max-width: 900px) {
    .hadith-list { flex-direction: column; gap: var(--space-6); }
    .book-filter-panel { flex: 1 1 auto; position: static; }
  }
</style>
