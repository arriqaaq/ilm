<script lang="ts">
  import { onMount } from 'svelte';
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { getHadiths, getHadithSharhPages, getCollections, getStats } from '$lib/api';
  import type {
    ApiHadith,
    ApiCollection,
    ApiNarratorSearchResult,
    PaginatedResponse,
    SharhPageRef,
  } from '$lib/types';
  import HadithCard from '$lib/components/hadith/HadithCard.svelte';
  import HadithFilters from '$lib/components/hadith/HadithFilters.svelte';
  import HadithToolbar from '$lib/components/hadith/HadithToolbar.svelte';
  import BookViewerModal from '$lib/components/reader/BookViewerModal.svelte';
  import Pagination from '$lib/components/common/Pagination.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import ListingHero from '$lib/components/layout/ListingHero.svelte';
  import TwoColumn from '$lib/components/layout/TwoColumn.svelte';

  let result: PaginatedResponse<ApiHadith> | null = $state(null);
  let loading = $state(true);
  let sharhMappings: Record<string, SharhPageRef> = $state({});
  let sharhTarget: { bookId: number; pageIndex: number; bookName: string; hadithNumber: number } | null = $state(null);
  let collections: ApiCollection[] = $state([]);
  let totalCount: number | null = $state(null);
  /**
   * Cached metadata for currently-selected narrators. The URL only carries
   * narrator slugs; we hold the rich `ApiNarratorSearchResult` objects in
   * memory so the sidebar can render names without re-querying after
   * navigation.
   */
  let narratorCache: ApiNarratorSearchResult[] = $state([]);

  // ── URL-derived state ──
  let currentPage = $derived(Number(page.url.searchParams.get('page')) || 1);
  let books = $derived(parseCsvNumbers(page.url.searchParams.get('books')));
  let nMin = $derived(parseOptionalNumber(page.url.searchParams.get('n_min')));
  let nMax = $derived(parseOptionalNumber(page.url.searchParams.get('n_max')));
  let narrators = $derived(parseCsvStrings(page.url.searchParams.get('narrators')));
  let q = $derived(page.url.searchParams.get('q') ?? '');
  let sort = $derived(
    (page.url.searchParams.get('sort') === 'number_desc'
      ? 'number_desc'
      : 'number_asc') as 'number_asc' | 'number_desc'
  );
  let view = $derived(
    (page.url.searchParams.get('view') === 'grid' ? 'grid' : 'list') as 'list' | 'grid'
  );

  function parseCsvNumbers(s: string | null): number[] {
    if (!s) return [];
    return s.split(',').map(p => Number(p.trim())).filter(n => Number.isFinite(n));
  }
  function parseCsvStrings(s: string | null): string[] {
    if (!s) return [];
    return s.split(',').map(p => p.trim()).filter(Boolean);
  }
  function parseOptionalNumber(s: string | null): number | undefined {
    if (s === null || s === '') return undefined;
    const n = Number(s);
    return Number.isFinite(n) ? n : undefined;
  }

  async function load() {
    loading = true;
    try {
      result = await getHadiths({
        books: books.length ? books : undefined,
        narrators: narrators.length ? narrators : undefined,
        n_min: nMin,
        n_max: nMax,
        q: q || undefined,
        sort,
        page: currentPage,
      });

      sharhMappings = {};
      if (result && result.data.length > 0) {
        // Sharh pages are scoped to a single book — only fetch when results
        // come from one collection. Multi-collection result sets skip the
        // batch.
        const uniqueBooks = new Set(result.data.map(h => h.collection_id));
        if (uniqueBooks.size === 1) {
          const numbers = result.data.map(h => h.hadith_number);
          const bookId = result.data[0].collection_id;
          getHadithSharhPages(bookId, numbers)
            .then(res => { sharhMappings = res.mappings; })
            .catch(() => {});
        }
      }
    } catch (e) {
      console.error('Failed to load hadiths:', e);
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    try { collections = await getCollections(); } catch (e) { console.error(e); }
    try { totalCount = (await getStats()).hadith_count; } catch (e) { /* ignore */ }
  });

  $effect(() => {
    void currentPage; void books; void nMin; void nMax;
    void narrators; void q; void sort;
    load();
  });

  // ── URL writes ──
  function buildSearch(overrides: Record<string, string | null> = {}): URLSearchParams {
    const sp = new URLSearchParams();
    if (books.length) sp.set('books', books.join(','));
    if (narrators.length) sp.set('narrators', narrators.join(','));
    if (nMin !== undefined) sp.set('n_min', String(nMin));
    if (nMax !== undefined) sp.set('n_max', String(nMax));
    if (q) sp.set('q', q);
    if (sort !== 'number_asc') sp.set('sort', sort);
    if (view !== 'list') sp.set('view', view);
    if (currentPage > 1) sp.set('page', String(currentPage));
    for (const [k, v] of Object.entries(overrides)) {
      if (v === null || v === '') sp.delete(k);
      else sp.set(k, v);
    }
    return sp;
  }

  function navigate(sp: URLSearchParams, opts: { replace?: boolean } = {}) {
    const qs = sp.toString();
    const url = qs ? `/hadiths?${qs}` : '/hadiths';
    goto(url, { keepFocus: true, noScroll: true, replaceState: opts.replace ?? false });
  }

  function changePage(newPage: number) {
    navigate(buildSearch({ page: newPage > 1 ? String(newPage) : null }));
  }

  function onFiltersChange(patch: {
    books?: number[];
    nMin?: number | undefined;
    nMax?: number | undefined;
    narrators?: string[];
    narratorDetails?: ApiNarratorSearchResult[];
  }) {
    if (patch.narratorDetails !== undefined) {
      narratorCache = patch.narratorDetails;
    }
    const overrides: Record<string, string | null> = { page: null };
    if ('books' in patch && patch.books !== undefined) {
      overrides.books = patch.books.length ? patch.books.join(',') : null;
    }
    if ('narrators' in patch && patch.narrators !== undefined) {
      overrides.narrators = patch.narrators.length ? patch.narrators.join(',') : null;
    }
    if ('nMin' in patch) {
      overrides.n_min = patch.nMin !== undefined ? String(patch.nMin) : null;
    }
    if ('nMax' in patch) {
      overrides.n_max = patch.nMax !== undefined ? String(patch.nMax) : null;
    }
    // Filter changes replace history so rapid typing doesn't pollute back-stack.
    navigate(buildSearch(overrides), { replace: true });
  }

  function onToolbarChange(patch: {
    q?: string;
    sort?: 'number_asc' | 'number_desc';
    view?: 'list' | 'grid';
  }) {
    const overrides: Record<string, string | null> = { page: null };
    if (patch.q !== undefined) overrides.q = patch.q || null;
    if (patch.sort !== undefined) {
      overrides.sort = patch.sort === 'number_asc' ? null : patch.sort;
    }
    if (patch.view !== undefined) {
      overrides.view = patch.view === 'list' ? null : patch.view;
    }
    // Toolbar tweaks (search query, sort, view) shouldn't pile up history entries.
    navigate(buildSearch(overrides), { replace: true });
  }

  // Keep narratorCache aligned with the URL — drop entries that disappeared
  // (e.g. via "Clear all" from a different control) and tolerate cold loads
  // where we have ids but no metadata yet.
  $effect(() => {
    const ids = new Set(narrators);
    if (narratorCache.length > 0) {
      narratorCache = narratorCache.filter(n => ids.has(n.id));
    }
  });

  const heroCount = $derived.by(() => {
    const c = totalCount;
    // While stats are loading we don't have a count yet — return a soft
    // placeholder rather than flashing a 0.
    return c === null ? 'Search across the canon' : `Search ${c.toLocaleString()} hadiths`;
  });
</script>

<div class="hadiths-page">
  <ListingHero eyebrow="Ḥadīth" title="Hadiths" subtitle={heroCount}>
    {#snippet description()}
      Browse and search the canonical Sunni hadith collections — Ṣaḥīḥ al-Bukhārī,
      Ṣaḥīḥ Muslim, and the four Sunan. Filter by collection, narrator, and hadith
      number to find exactly the chain or text you need.
    {/snippet}
  </ListingHero>

  <div class="hadiths-content">
    <TwoColumn sidebarWidth={280} sticky sidebarSide="left">
      {#snippet sidebar()}
        <HadithFilters
          {collections}
          {books}
          {nMin}
          {nMax}
          {narrators}
          narratorDetails={narratorCache}
          onChange={onFiltersChange}
        />
      {/snippet}
      {#snippet main()}
        <HadithToolbar {q} {sort} {view} onChange={onToolbarChange} />

        {#if loading}
          <LoadingSpinner />
        {:else if result && result.data.length > 0}
          <div class="results results--{view}">
            {#each result.data as hadith (hadith.id)}
              <HadithCard
                {hadith}
                {view}
                sharhPage={sharhMappings[String(hadith.hadith_number)]}
                onopensharh={(info) => { sharhTarget = info; }}
              />
            {/each}
          </div>
          <Pagination page={result.page} hasMore={result.has_more} onPageChange={changePage} />
        {:else}
          <div class="empty">No hadiths match these filters.</div>
        {/if}
      {/snippet}
    </TwoColumn>
  </div>
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
  /* Outer wrapper has no padding — the hero band is full-bleed and the
     content container handles its own. */
  .hadiths-page { width: 100%; }

  .hadiths-content {
    max-width: var(--page-width);
    margin: 0 auto;
    padding: var(--space-10) var(--space-6) var(--space-12);
  }
  @media (min-width: 768px) {
    .hadiths-content { padding-top: var(--space-12); }
  }
  @media (max-width: 640px) {
    .hadiths-content { padding: var(--space-6) var(--space-4) var(--space-8); }
  }

  .results {
    display: flex;
    flex-direction: column;
    gap: 0;
  }
  .results--grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: var(--space-4);
  }

  .empty {
    text-align: center;
    color: var(--text-muted);
    padding: var(--space-10);
  }
</style>
