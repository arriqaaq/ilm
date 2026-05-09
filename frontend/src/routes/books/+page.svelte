<script lang="ts">
  import { onMount } from 'svelte';
  import { getBooksList, getCollections } from '$lib/api';
  import type { Book, ApiCollection } from '$lib/types';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import PageHeader from '$lib/components/common/PageHeader.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import CollectionCard from '$lib/components/landing/CollectionCard.svelte';
  import TabStrip from '$lib/components/layout/TabStrip.svelte';

  type ColorVariant = 'walnut' | 'sienna' | 'malachite' | 'saffron' | 'lapis' | 'aubergine';
  type PatternId = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10;

  const COLLECTION_VARIANTS: { color: ColorVariant; pattern: PatternId }[] = [
    { color: 'malachite', pattern: 1 },
    { color: 'sienna',    pattern: 2 },
    { color: 'lapis',     pattern: 3 },
    { color: 'saffron',   pattern: 4 },
    { color: 'walnut',    pattern: 5 },
    { color: 'aubergine', pattern: 6 },
  ];

  const TABS = [
    { id: 'all',        label: 'All' },
    { id: 'tafsir',     label: 'Tafsir' },
    { id: 'sharh',      label: 'Sharḥ' },
    { id: 'collection', label: 'Sunan Editions' },
    { id: 'grading',    label: 'Hadith Grading' },
    { id: 'biography',  label: 'Narrator Bios' },
  ] as const;
  type TabId = (typeof TABS)[number]['id'];

  let activeTab: TabId = $state('all');
  let collections: ApiCollection[] = $state([]);
  let books: Book[] = $state([]);
  let loadingBooks = $state(true);

  async function loadTab(tab: TabId) {
    loadingBooks = true;
    const bookType = tab === 'all' ? undefined : tab;
    try {
      books = await getBooksList(undefined, bookType);
    } catch (e) {
      console.error('Failed to load books:', e);
      books = [];
    } finally {
      loadingBooks = false;
    }
  }

  $effect(() => { loadTab(activeTab); });

  onMount(async () => {
    try {
      collections = await getCollections();
    } catch (e) {
      console.error('Failed to load collections:', e);
    }
  });
</script>

<div class="page-shell">
  <PageHeader
    eyebrow="Library"
    title="Books"
    subtitle="Six canonical hadith collections plus every classical reference ingested in this project."
  />

  {#if collections.length > 0}
    <section class="collections-section">
      <div class="section-eyebrow"><Eyebrow>Major Hadith Collections</Eyebrow></div>
      <div class="collections-grid">
        {#each collections as c, i (c.id)}
          {@const v = COLLECTION_VARIANTS[i % COLLECTION_VARIANTS.length]}
          <CollectionCard
            title={c.name_en}
            subtitle={c.name_ar ?? undefined}
            color={v.color}
            pattern={v.pattern}
            href={`/hadiths?book=${c.collection_id}`}
          />
        {/each}
      </div>
    </section>
  {/if}

  <section class="library-section">
    <div class="section-eyebrow"><Eyebrow>Reference Library</Eyebrow></div>

    <TabStrip
      ariaLabel="Reference library"
      bind:active={activeTab}
      tabs={TABS as unknown as { id: TabId; label: string }[]}
    />

    {#if loadingBooks}
      <LoadingSpinner />
    {:else if books.length > 0}
      <div class="book-list">
        {#each books as book (book.book_id)}
          <a href={`/books/${book.book_id}`} class="book-row">
            <span class="book-num mono">#{book.book_id}</span>
            <div class="book-meta">
              <h3 class="book-name-ar arabic-prose" dir="rtl">{book.name_ar}</h3>
              {#if book.name_en && book.name_en !== book.name_ar}
                <p class="book-name-en">{book.name_en}</p>
              {/if}
              <p class="book-sub">
                {#if book.author_ar}<span class="book-author arabic-prose" dir="rtl">{book.author_ar}</span>{/if}
                {#if book.author_ar && book.total_pages}<span class="dot">·</span>{/if}
                {#if book.total_pages}<span class="pages">{book.total_pages.toLocaleString()} pages</span>{/if}
              </p>
            </div>
          </a>
        {/each}
      </div>
    {:else}
      <div class="empty">No books in this category yet.</div>
    {/if}
  </section>
</div>

<style>
  .collections-section { margin-bottom: var(--space-12); }
  .library-section { margin-bottom: var(--space-8); }
  .section-eyebrow { margin-bottom: var(--space-4); }

  .collections-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(176px, 1fr));
    gap: var(--space-4);
    justify-items: center;
  }

  .book-list {
    margin-top: var(--space-5);
    display: flex;
    flex-direction: column;
  }
  .book-row {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-2);
    border-bottom: 1px solid var(--border-subtle);
    color: inherit;
    text-decoration: none;
    transition: background var(--transition);
  }
  .book-row:hover { background: var(--bg-hover); }
  .book-row:last-child { border-bottom: none; }

  .book-num {
    flex-shrink: 0;
    font-size: var(--text-meta);
    color: var(--text-muted);
    line-height: 1.5;
    min-width: 3.5em;
  }
  .book-meta {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .book-name-ar {
    margin: 0;
    font-size: 1.1rem;
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    line-height: 1.4;
  }
  .book-name-en {
    margin: 0;
    font-family: var(--font-serif);
    font-size: var(--text-meta);
    color: var(--text-secondary);
    font-style: italic;
  }
  .book-sub {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--text-meta);
    color: var(--text-muted);
  }
  .book-author { color: var(--text-muted); }
  .dot { color: var(--text-muted); opacity: 0.6; }
  .pages { font-family: var(--font-mono); font-size: var(--text-2xs); }

  .empty {
    text-align: center;
    color: var(--text-muted);
    padding: var(--space-12);
    font-family: var(--font-serif);
    font-style: italic;
  }
</style>
