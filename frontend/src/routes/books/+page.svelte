<script lang="ts">
  import { onMount } from 'svelte';
  import { getBooksList, getCollections } from '$lib/api';
  import type { Book, ApiCollection } from '$lib/types';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  // Tab definitions filter the supplementary book table by `book_type`
  // (the genre/role distinction). The six canonical hadith collections
  // (Bukhari … Ibn Majah) live in the `collection` table, NOT `book`,
  // so they're displayed as a separate section above the tabs.
  const TABS: Array<{ key: string | null; label: string }> = [
    { key: null,         label: 'All' },
    { key: 'tafsir',     label: 'Tafsir' },
    { key: 'sharh',      label: 'Sharh' },
    { key: 'collection', label: 'Sunan Editions' },
    { key: 'grading',    label: 'Hadith Grading' },
    { key: 'biography',  label: 'Narrator Bios' },
  ];

  let activeTab: string | null = $state(null);
  let collections: ApiCollection[] = $state([]);
  let books: Book[] = $state([]);
  let loadingBooks = $state(true);

  async function loadTab(bookType: string | null) {
    loadingBooks = true;
    activeTab = bookType;
    try {
      books = await getBooksList(undefined, bookType ?? undefined);
    } catch (e) {
      console.error('Failed to load books:', e);
      books = [];
    } finally {
      loadingBooks = false;
    }
  }

  onMount(async () => {
    try {
      collections = await getCollections();
    } catch (e) {
      console.error('Failed to load collections:', e);
    }
    await loadTab(null);
  });
</script>

<div class="books-page">
  <div class="page-header">
    <h1>Library</h1>
    <p class="page-subtitle">Six canonical collections plus every classical book ingested in this project.</p>
  </div>

  <!-- Section 1: the six canonical hadith collections -->
  {#if collections.length > 0}
    <h2 class="section-title">Major Hadith Collections</h2>
    <div class="collections-grid">
      {#each collections as c (c.id)}
        <a href={`/hadiths?book=${c.collection_id}`} class="collection-card">
          <div class="collection-num">{c.collection_id}</div>
          <h3 class="book-title arabic" dir="rtl">{c.name_ar || c.name_en}</h3>
          {#if c.name_ar && c.name_en}
            <span class="book-en">{c.name_en}</span>
          {/if}
        </a>
      {/each}
    </div>
  {/if}

  <!-- Section 2: book_type-tabbed library -->
  <h2 class="section-title">Books</h2>
  <div class="tabs">
    {#each TABS as tab}
      <button
        class="tab"
        class:active={activeTab === tab.key}
        onclick={() => loadTab(tab.key)}
      >{tab.label}</button>
    {/each}
  </div>

  {#if loadingBooks}
    <LoadingSpinner />
  {:else}
    <div class="books-grid">
      {#each books as book (book.book_id)}
        <a href={`/books/${book.book_id}`} class="book-card">
          <h3 class="book-title arabic" dir="rtl">{book.name_ar}</h3>
          {#if book.name_en && book.name_en !== book.name_ar}
            <span class="book-en">{book.name_en}</span>
          {/if}
          {#if book.author_ar}
            <span class="book-author" dir="rtl">{book.author_ar}</span>
          {/if}
          {#if book.total_pages}
            <span class="book-meta">{book.total_pages} pages</span>
          {/if}
        </a>
      {/each}
    </div>
    {#if books.length === 0}
      <div class="empty">No books in this category yet.</div>
    {/if}
  {/if}
</div>

<style>
  .books-page {
    padding: 32px;
    max-width: 1100px;
    margin: 0 auto;
  }
  .page-header { margin-bottom: 28px; }
  .page-header h1 { font-size: 1.6rem; font-weight: 700; margin-bottom: 4px; }
  .page-subtitle { font-size: 0.85rem; color: var(--text-muted); }

  .section-title {
    font-size: 1.05rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 12px;
  }

  /* ── Major Hadith Collections (top section) ── */
  .collections-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 14px;
    margin-bottom: 32px;
  }
  .collection-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 10px;
    padding: 20px 18px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 16px;
    color: var(--text-primary);
    text-decoration: none;
    transition: all 0.25s ease;
  }
  .collection-card:hover {
    border-color: var(--accent);
    box-shadow: 0 8px 32px rgba(214,51,132,0.08);
    transform: translateY(-2px);
  }
  .collection-num {
    width: 36px; height: 36px;
    display: flex; align-items: center; justify-content: center;
    background: var(--accent-muted);
    color: var(--accent);
    border-radius: 50%;
    font-weight: 700;
    font-size: 0.85rem;
    font-family: var(--font-mono);
  }

  /* ── Book-type tabs ── */
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 20px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 8px;
  }
  .tab {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 6px 14px;
    font-size: 0.85rem;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition);
  }
  .tab:hover { border-color: var(--accent); color: var(--accent); }
  .tab.active {
    background: var(--accent-muted);
    color: var(--accent);
    border-color: var(--accent);
  }

  /* ── Library grid ── */
  .books-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
  }
  .book-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 22px 20px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: 14px;
    color: var(--text-primary);
    text-decoration: none;
    transition: all 0.25s ease;
    position: relative;
    overflow: hidden;
  }
  .book-card::before {
    content: '';
    position: absolute;
    top: 0; left: 0; right: 0;
    height: 3px;
    background: var(--accent);
    opacity: 0;
    transition: opacity 0.25s ease;
  }
  .book-card:hover {
    border-color: var(--accent);
    box-shadow: 0 8px 32px rgba(214,51,132,0.08);
    transform: translateY(-3px);
  }
  .book-card:hover::before { opacity: 1; }

  .book-title {
    font-size: 1.05rem;
    font-weight: 600;
    line-height: 1.7;
    color: var(--text-primary);
  }
  .book-en { font-size: 0.78rem; color: var(--text-muted); }
  .book-author { font-size: 0.8rem; color: var(--text-secondary); }
  .book-meta {
    font-size: 0.7rem;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .empty {
    text-align: center;
    color: var(--text-muted);
    padding: 60px;
    font-size: 0.9rem;
  }

  @media (max-width: 600px) {
    .books-page { padding: 20px; }
    .collections-grid { grid-template-columns: 1fr 1fr; }
    .books-grid { grid-template-columns: 1fr; gap: 12px; }
  }
</style>
