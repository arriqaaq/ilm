<script lang="ts">
  import { page } from '$app/state';
  import type { UserNote } from '$lib/types';
  import { fetchNote, updateNote, updateRefAnnotation, removeRefFromNote } from '$lib/api';
  import NoteEditor from '$lib/components/notes/NoteEditor.svelte';
  import RefCard from '$lib/components/notes/RefCard.svelte';
  import TagInput from '$lib/components/notes/TagInput.svelte';
  import Badge from '$lib/components/common/Badge.svelte';
  import NotebookPicker from '$lib/components/notes/NotebookPicker.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import SectionHeading from '$lib/components/common/SectionHeading.svelte';
  import Divider from '$lib/components/common/Divider.svelte';

  let note: UserNote | null = $state(null);
  let loading = $state(true);
  let editingTitle = $state(false);
  let titleInput = $state('');

  let id = $derived(page.params.id);

  $effect(() => {
    if (!id) return;
    loading = true;
    fetchNote(id)
      .then(n => {
        note = n;
        titleInput = n.title ?? '';
      })
      .catch(() => { note = null; })
      .finally(() => { loading = false; });
  });

  async function handleTitleSave() {
    if (!note) return;
    const updated = await updateNote(note.id, { title: titleInput.trim() });
    note = updated;
    editingTitle = false;
  }

  async function handleContentSave(data: { content: string; color: string; tags: string[] }) {
    if (!note) return;
    const updated = await updateNote(note.id, {
      content: data.content,
      color: data.color,
      tags: data.tags,
    });
    note = updated;
  }

  async function handleTagsChange(tags: string[]) {
    if (!note) return;
    const updated = await updateNote(note.id, { tags });
    note = updated;
  }

  async function handleRefAnnotation(idx: number, annotation: string) {
    if (!note) return;
    const updated = await updateRefAnnotation(note.id, idx, annotation);
    note = updated;
  }

  async function handleRemoveRef(idx: number) {
    if (!note || !note.refs[idx]) return;
    const updated = await removeRefFromNote(note.id, note.refs[idx]);
    note = updated;
  }

  let selectedNotebookId: string | null = $state(null);

  $effect(() => {
    if (note) selectedNotebookId = note.notebook_id ?? null;
  });

  async function handleNotebookChange() {
    if (!note || selectedNotebookId === (note.notebook_id ?? null)) return;
    const updated = await updateNote(note.id, { notebook_id: selectedNotebookId ?? undefined });
    note = updated;
  }

  let sourceLabel = $derived.by(() => {
    if (!note) return '';
    const hasAyah = note.refs.some(r => r.ref_type === 'ayah');
    const hasHadith = note.refs.some(r => r.ref_type === 'hadith');
    if (hasAyah && hasHadith) return 'Quran + Hadith';
    if (hasAyah) return 'Quran';
    if (hasHadith) return 'Hadith';
    return 'Topic';
  });
</script>

<div class="note-detail">
  {#if loading}
    <div class="state">Loading note…</div>
  {:else if !note}
    <div class="state">Note not found.</div>
  {:else}
    <header class="title-area">
      <div class="title-top">
        <Eyebrow>Note · {sourceLabel}</Eyebrow>
        <a href="/notes" class="back-link">← All notes</a>
      </div>

      {#if editingTitle}
        <input
          class="title-input"
          bind:value={titleInput}
          onblur={handleTitleSave}
          onkeydown={(e) => { if (e.key === 'Enter') handleTitleSave(); }}
        />
      {:else}
        <h1 class="title">
          <button type="button" class="title-btn" onclick={() => { editingTitle = true; }}>
            {note.title ?? 'Untitled Note'}
            <span class="edit-hint">✎</span>
          </button>
        </h1>
      {/if}

      <div class="title-meta">
        <Badge text={sourceLabel} variant="default" />
        <span class="ref-count">{note.refs.length} references</span>
        <NotebookPicker bind:value={selectedNotebookId} onchange={handleNotebookChange} />
      </div>
    </header>

    <hr class="separator" />

    <div class="tags-area">
      <TagInput tags={note.tags} onchange={handleTagsChange} />
    </div>

    <section class="section">
      <SectionHeading eyebrow="Notes" title="Overall Notes" level={2} />
      <NoteEditor
        note={note}
        onsave={handleContentSave}
      />
    </section>

    {#if note.refs.length > 0}
      <Divider variant="ornamental" />
      <section class="section">
        <SectionHeading eyebrow="Refs" title="Collected References ({note.refs.length})" level={2} />
        <div class="refs-list">
          {#each note.refs as ref, idx}
            <RefCard
              {ref}
              onupdateannotation={(ann) => handleRefAnnotation(idx, ann)}
              onremove={() => handleRemoveRef(idx)}
            />
          {/each}
        </div>
      </section>
    {/if}
  {/if}
</div>

<style>
  .note-detail {
    max-width: 720px;
    margin: 0 auto;
    padding: var(--space-8) var(--space-6) var(--space-12);
  }

  .state {
    text-align: center;
    padding: var(--space-12);
    color: var(--text-muted);
    font-family: var(--font-serif);
    font-style: italic;
  }

  .title-area { margin-bottom: var(--space-3); }
  .title-top {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-3);
  }
  .back-link {
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-muted);
    text-decoration: none;
    transition: color var(--transition);
  }
  .back-link:hover { color: var(--accent); }

  .title {
    font-family: var(--font-serif);
    font-size: 2.1rem;
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-tight);
    line-height: 1.2;
    margin: 0;
  }
  .title-btn {
    all: unset;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font: inherit;
    letter-spacing: inherit;
    color: inherit;
  }
  .edit-hint {
    font-size: var(--text-meta);
    color: var(--text-muted);
    opacity: 0;
    transition: opacity var(--transition);
  }
  .title:hover .edit-hint { opacity: 1; }
  .title-input {
    font-family: var(--font-serif);
    font-size: 2.1rem;
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-tight);
    border: none;
    border-bottom: 2px solid var(--accent);
    background: transparent;
    color: var(--text-primary);
    width: 100%;
    outline: none;
    padding: var(--space-1) 0;
  }

  .title-meta {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-top: var(--space-3);
    flex-wrap: wrap;
  }
  .ref-count {
    font-family: var(--font-mono);
    font-size: var(--text-meta);
    color: var(--text-muted);
  }

  .separator {
    border: none;
    border-top: 1px solid var(--border-subtle);
    margin: var(--space-6) 0;
  }
  .tags-area { margin-bottom: var(--space-6); }
  .section { margin-bottom: var(--space-8); }

  .refs-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
</style>
