<script lang="ts">
  import type { UserNote } from '$lib/types';
  import { fetchAllNotes, fetchNoteTags, deleteNote, exportNotes } from '$lib/api';
  import NoteCard from '$lib/components/notes/NoteCard.svelte';
  import NoteModal from '$lib/components/notes/NoteModal.svelte';
  import NotebookSidebar from '$lib/components/notes/NotebookSidebar.svelte';

  const COLORS = ['yellow', 'green', 'blue', 'pink', 'purple'] as const;

  let notes: UserNote[] = $state([]);
  let allTags: string[] = $state([]);
  let showNewNote = $state(false);
  let loading = $state(true);
  let searchQuery = $state('');
  let activeColor = $state<string | null>(null);
  let activeTag = $state<string | null>(null);
  let activeNotebookId = $state<string | null>(null);
  let page = $state(1);
  let hasMore = $state(false);

  $effect(() => {
    fetchNoteTags().then(t => { allTags = t; }).catch(() => {});
  });

  $effect(() => {
    const _color = activeColor;
    const _tag = activeTag;
    const _q = searchQuery;
    const _page = page;
    const _nb = activeNotebookId;

    loading = true;
    const params: Record<string, string | number> = { page: _page, limit: 20 };
    if (_color) params.color = _color;
    if (_tag) params.tag = _tag;
    if (_q.trim()) params.q = _q.trim();
    if (_nb) params.notebook_id = _nb;

    fetchAllNotes(params as any)
      .then(res => {
        if (_page === 1) {
          notes = res.data;
        } else {
          notes = [...notes, ...res.data];
        }
        hasMore = res.has_more;
      })
      .catch(() => { if (_page === 1) notes = []; })
      .finally(() => { loading = false; });
  });

  function setColor(color: string | null) {
    activeColor = activeColor === color ? null : color;
    page = 1;
  }

  function setTag(tag: string | null) {
    activeTag = activeTag === tag ? null : tag;
    page = 1;
  }

  async function handleDelete(note: UserNote) {
    await deleteNote(note.id);
    notes = notes.filter(n => n.id !== note.id);
  }

  function handleNoteSaved() {
    showNewNote = false;
    page = 1;
    fetchNoteTags().then(t => { allTags = t; }).catch(() => {});
  }

  async function handleExport() {
    const data = await exportNotes();
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `ilm-notes-${new Date().toISOString().slice(0, 10)}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

</script>

<div class="notes-layout">
  <NotebookSidebar bind:activeNotebookId />

  <div class="notes-page">
    <div class="page-header">
      <h1>Notes</h1>
      <div class="header-actions">
        <button class="btn-new" onclick={() => { showNewNote = !showNewNote; }}>+ New Note</button>
        <button class="btn-export" onclick={handleExport}>Export</button>
      </div>
    </div>

    {#if showNewNote}
      <NoteModal
        onclose={() => { showNewNote = false; }}
        onsaved={handleNoteSaved}
      />
    {/if}

    <div class="search-bar">
      <input
        type="text"
        placeholder="Search notes..."
        bind:value={searchQuery}
        oninput={() => { page = 1; }}
      />
    </div>

    <div class="filters">
      {#if allTags.length > 0}
        <div class="tag-filters">
          {#each allTags as tag}
            <button
              class="tag-chip"
              class:active={activeTag === tag}
              onclick={() => setTag(tag)}
            >
              {tag}
            </button>
          {/each}
        </div>
      {/if}

      <div class="color-filters">
        {#each COLORS as color}
          <button
            class="color-dot"
            class:active={activeColor === color}
            style="background: var(--note-{color})"
            onclick={() => setColor(color)}
            aria-label="Filter by {color}"
          ></button>
        {/each}
      </div>

      {#if activeTag || activeColor}
        <button class="clear-filters" onclick={() => { activeTag = null; activeColor = null; page = 1; }}>
          Clear filters
        </button>
      {/if}
    </div>

    {#if loading}
      <div class="loading">Loading notes...</div>
    {:else if notes.length === 0}
      <div class="empty">
        <div class="empty-icon-wrap">
          <span class="empty-icon">&#9998;</span>
        </div>
        {#if activeTag || activeColor || searchQuery}
          <div class="empty-text">No notes match your filters</div>
        {:else}
          <div class="empty-text">No notes yet</div>
          <div class="empty-hint">Add notes from the Quran or Hadith pages, or create a new study note above</div>
        {/if}
      </div>
    {:else}
      <div class="notes-list">
        {#each notes as note (note.id)}
          <a href="/notes/{note.id}" class="note-link">
            <NoteCard {note} ondelete={handleDelete} />
          </a>
        {/each}
      </div>

      {#if hasMore}
        <button class="load-more" onclick={() => { page++; }}>
          Load more
        </button>
      {/if}
    {/if}
  </div>
</div>

<style>
  .notes-layout {
    display: flex;
    min-height: calc(100vh - var(--topbar-height, 56px));
  }
  .notes-page {
    flex: 1;
    padding: 40px 40px 60px;
    max-width: 960px;
    min-width: 0;
  }
  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 36px;
  }
  .page-header h1 {
    font-family: var(--font-sans);
    font-size: 2.2rem;
    font-weight: 800;
    letter-spacing: -0.02em;
  }
  .header-actions {
    display: flex;
    gap: 10px;
  }
  .btn-new {
    padding: 0 var(--btn-padding-x-md);
    height: var(--btn-height-md);
    font-size: var(--btn-font-size);
    font-weight: var(--btn-font-weight);
    text-transform: uppercase;
    letter-spacing: var(--btn-letter-spacing);
    color: var(--btn-primary-fg);
    background: var(--accent);
    border: none;
    border-radius: var(--btn-radius);
    cursor: pointer;
    transition: all var(--transition);
  }
  .btn-new:hover {
    background: var(--accent-hover);
  }
  .btn-export {
    padding: 10px 16px;
    font-size: 0.8rem;
    color: var(--text-secondary);
    background: var(--btn-bg);
    border: 1px solid var(--btn-border);
    border-radius: var(--radius);
    cursor: pointer;
    transition: all var(--transition);
  }
  .btn-export:hover {
    border-color: var(--btn-border-hover);
    background: var(--btn-bg-hover);
  }

  .search-bar {
    margin-bottom: 20px;
    position: relative;
  }
  .search-bar::before {
    content: '';
    position: absolute;
    left: 18px;
    top: 50%;
    transform: translateY(-50%);
    width: 16px;
    height: 16px;
    background: var(--text-muted);
    mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='11' cy='11' r='8'/%3E%3Cpath d='m21 21-4.3-4.3'/%3E%3C/svg%3E");
    -webkit-mask-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='16' height='16' viewBox='0 0 24 24' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Ccircle cx='11' cy='11' r='8'/%3E%3Cpath d='m21 21-4.3-4.3'/%3E%3C/svg%3E");
    pointer-events: none;
    opacity: 0.5;
    z-index: 1;
  }
  .search-bar input {
    width: 100%;
    padding: 12px 20px 12px 48px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 0.9rem;
    font-family: var(--font-sans);
    outline: none;
    box-sizing: border-box;
    transition: border-color var(--transition);
  }
  .search-bar input::placeholder {
    color: var(--text-muted);
  }
  .search-bar input:focus {
    border-color: var(--accent);
  }

  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 12px;
    align-items: center;
    margin-bottom: 24px;
  }
  .tag-filters {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .tag-chip {
    padding: 6px 16px;
    font-size: 0.78rem;
    font-weight: 600;
    border: 1.5px solid var(--border);
    border-radius: 20px;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition);
  }
  .tag-chip:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .tag-chip.active {
    background: var(--accent);
    color: var(--btn-primary-fg);
    border-color: var(--accent);
  }
  .color-filters {
    display: flex;
    gap: 8px;
  }
  .color-dot {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 2px solid transparent;
    cursor: pointer;
    transition: all var(--transition);
    padding: 0;
    box-shadow: 0 1px 4px rgba(0,0,0,0.1);
  }
  .color-dot:hover {
    transform: scale(1.15);
  }
  .color-dot.active {
    border-color: var(--accent);
    transform: scale(1.2);
    box-shadow: 0 0 0 4px rgba(200,169,106,0.15);
  }
  .clear-filters {
    font-size: 0.75rem;
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    transition: color var(--transition);
  }
  .clear-filters:hover { color: var(--accent); }

  .notes-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: 16px;
  }
  .note-link {
    text-decoration: none;
    color: inherit;
    display: block;
  }
  .loading {
    text-align: center;
    padding: 60px;
    color: var(--text-muted);
    font-family: var(--font-serif);
    font-style: italic;
  }
  .empty {
    text-align: center;
    padding: 100px 32px;
    color: var(--text-muted);
    background: var(--bg-secondary);
    border-radius: var(--radius-xl);
    border: 1px solid var(--border-subtle);
  }
  .empty-icon-wrap {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 80px;
    height: 80px;
    border-radius: 50%;
    background: var(--accent-muted);
    margin-bottom: 20px;
  }
  .empty-icon {
    font-size: 2.5rem;
    opacity: 0.6;
  }
  .empty-text {
    font-family: var(--font-serif);
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 10px;
  }
  .empty-hint {
    font-family: var(--font-serif);
    font-size: 0.9rem;
    font-style: italic;
    line-height: 1.6;
    max-width: 420px;
    margin: 0 auto;
  }
  .load-more {
    display: block;
    margin: 24px auto;
    padding: 12px 36px;
    font-size: 0.8rem;
    font-family: var(--font-sans);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--accent);
    background: none;
    border: 1px solid var(--accent);
    border-radius: var(--radius);
    cursor: pointer;
    transition: all var(--transition);
  }
  .load-more:hover {
    background: var(--accent-muted);
  }

  @media (max-width: 768px) {
    .notes-layout {
      flex-direction: column;
    }
    .notes-page {
      padding: 20px 16px 40px;
    }
    .notes-list {
      grid-template-columns: 1fr;
    }
  }
</style>
