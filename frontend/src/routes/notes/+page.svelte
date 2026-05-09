<script lang="ts">
  import type { UserNote } from '$lib/types';
  import { fetchAllNotes, fetchNoteTags, deleteNote, exportNotes } from '$lib/api';
  import NoteCard from '$lib/components/notes/NoteCard.svelte';
  import NoteModal from '$lib/components/notes/NoteModal.svelte';
  import NotebookSidebar from '$lib/components/notes/NotebookSidebar.svelte';
  import PageHeader from '$lib/components/common/PageHeader.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import Button from '$lib/components/common/Button.svelte';
  import Ornament from '$lib/components/common/Ornament.svelte';

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

  <main class="notes-main">
    <PageHeader
      eyebrow="Notes"
      title="My Notes"
      subtitle="Annotations and study collections from the Qurʾān and Sunnah."
    />

    <div class="toolbar">
      <input
        class="search-input"
        type="text"
        placeholder="Search notes…"
        bind:value={searchQuery}
        oninput={() => { page = 1; }}
      />
      <Button variant="primary" size="md" onclick={() => { showNewNote = !showNewNote; }}>+ New</Button>
      <Button variant="soft" size="md" onclick={handleExport}>Export</Button>
    </div>

    {#if showNewNote}
      <NoteModal
        onclose={() => { showNewNote = false; }}
        onsaved={handleNoteSaved}
      />
    {/if}

    <div class="filter-band">
      <div class="filter-eyebrow"><Eyebrow tone="muted">Filter</Eyebrow></div>
      <div class="filter-row">
        {#if allTags.length > 0}
          <div class="tag-chips">
            {#each allTags as tag}
              <button
                class="chip"
                class:active={activeTag === tag}
                onclick={() => setTag(tag)}
              >{tag}</button>
            {/each}
          </div>
        {/if}

        <div class="color-dots">
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
            Clear
          </button>
        {/if}
      </div>
    </div>

    {#if loading}
      <div class="loading">Loading notes…</div>
    {:else if notes.length === 0}
      <div class="empty">
        <Ornament variant="star" size={28} color="var(--accent)" />
        {#if activeTag || activeColor || searchQuery}
          <div class="empty-text">No notes match your filters</div>
        {:else}
          <div class="empty-text">No notes yet</div>
          <div class="empty-hint">Add notes from the Qurʾān or Ḥadīth pages, or create a new study note above.</div>
        {/if}
      </div>
    {:else}
      <div class="notes-grid">
        {#each notes as note (note.id)}
          <a href="/notes/{note.id}" class="note-link">
            <NoteCard {note} ondelete={handleDelete} />
          </a>
        {/each}
      </div>

      {#if hasMore}
        <button class="load-more" onclick={() => { page++; }}>Load more</button>
      {/if}
    {/if}
  </main>
</div>

<style>
  .notes-layout {
    display: flex;
    min-height: 100%;
  }
  .notes-main {
    flex: 1;
    min-width: 0;
    max-width: 1080px;
    margin: 0 auto;
    padding: var(--space-8) var(--space-6) var(--space-12);
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-5);
  }
  .search-input {
    flex: 1;
    max-width: 480px;
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    outline: none;
    transition: border-color var(--transition);
  }
  .search-input:focus { border-color: var(--accent); }
  .search-input::placeholder { color: var(--text-muted); }

  .filter-band { margin-bottom: var(--space-6); }
  .filter-eyebrow { margin-bottom: var(--space-2); }
  .filter-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    align-items: center;
  }
  .tag-chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .chip {
    padding: var(--space-1) var(--space-3);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-medium);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition);
  }
  .chip:hover {
    background: var(--bg-hover);
    border-color: var(--btn-border-hover);
  }
  .chip.active {
    background: var(--accent-muted);
    color: var(--accent);
    border-color: var(--accent);
  }

  .color-dots {
    display: flex;
    gap: var(--space-2);
    margin-left: var(--space-2);
  }
  .color-dot {
    width: 24px;
    height: 24px;
    border-radius: var(--radius-full);
    border: 2px solid transparent;
    cursor: pointer;
    transition: all var(--transition);
    padding: 0;
  }
  .color-dot:hover { transform: scale(1.12); }
  .color-dot.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-muted);
  }

  .clear-filters {
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    transition: color var(--transition);
  }
  .clear-filters:hover { color: var(--accent); }

  .notes-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
    gap: var(--space-4);
  }
  .note-link { text-decoration: none; color: inherit; display: block; }

  .loading {
    text-align: center;
    padding: var(--space-12);
    color: var(--text-muted);
    font-family: var(--font-serif);
    font-style: italic;
  }
  .empty {
    text-align: center;
    padding: var(--space-12) var(--space-6);
    color: var(--text-muted);
  }
  .empty :global(.ornament) { margin-bottom: var(--space-4); }
  .empty-text {
    font-family: var(--font-serif);
    font-size: var(--text-lead);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    margin-bottom: var(--space-2);
  }
  .empty-hint {
    font-family: var(--font-serif);
    font-size: var(--text-body);
    font-style: italic;
    line-height: 1.6;
    max-width: 420px;
    margin: 0 auto;
    color: var(--text-muted);
  }

  .load-more {
    display: block;
    margin: var(--space-6) auto 0;
    padding: var(--space-3) var(--space-6);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
    color: var(--accent);
    background: none;
    border: 1px solid var(--accent);
    border-radius: var(--radius-pill);
    cursor: pointer;
    transition: all var(--transition);
  }
  .load-more:hover { background: var(--accent-muted); }

  @media (max-width: 768px) {
    .notes-layout { flex-direction: column; }
    .notes-main { padding: var(--space-5) var(--space-4) var(--space-10); }
    .notes-grid { grid-template-columns: 1fr; }
  }
</style>
