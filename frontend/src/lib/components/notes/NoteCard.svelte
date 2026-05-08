<script lang="ts">
  import type { UserNote } from '$lib/types';
  import { parseContent } from '$lib/editor';
  import Badge from '$lib/components/common/Badge.svelte';
  import MentionChip from './MentionChip.svelte';
  import EmbeddedRef from './EmbeddedRef.svelte';
  import LinkPreviewCard from './LinkPreviewCard.svelte';

  let { note, onedit, ondelete }: {
    note: UserNote;
    onedit?: (note: UserNote) => void;
    ondelete?: (note: UserNote) => void;
  } = $props();

  let sourceLabel = $derived.by(() => {
    if (note.ref_type === 'topic') {
      const hasAyah = note.refs.some(r => r.ref_type === 'ayah');
      const hasHadith = note.refs.some(r => r.ref_type === 'hadith');
      if (hasAyah && hasHadith) return 'Quran + Hadith';
      if (hasAyah) return 'Quran';
      if (hasHadith) return 'Hadith';
      return 'Topic';
    }
    return note.ref_type === 'ayah' ? 'Quran' : 'Hadith';
  });

  let refLabel = $derived(
    note.ref_type === 'ayah' ? `Quran ${note.ref_id}`
      : note.ref_type === 'hadith' ? `Hadith ${note.ref_id}`
      : null
  );

  let timeAgo = $derived.by(() => {
    const d = new Date(note.updated_at);
    const now = Date.now();
    const diff = now - d.getTime();
    const mins = Math.floor(diff / 60000);
    if (mins < 1) return 'just now';
    if (mins < 60) return `${mins}m ago`;
    const hrs = Math.floor(mins / 60);
    if (hrs < 24) return `${hrs}h ago`;
    const days = Math.floor(hrs / 24);
    if (days < 30) return `${days}d ago`;
    return d.toLocaleDateString();
  });

  let contentParts = $derived(parseContent(note.content));
</script>

<div class="card card-lg note-card" style="--card-accent: var(--note-{note.color}); --glow-rgb: var(--note-{note.color}-rgb)">
  <div class="note-header">
    <div class="note-meta">
      {#if note.title}
        <span class="note-title">{note.title}</span>
      {:else if refLabel}
        <a href={note.ref_type === 'ayah' ? `/quran/${note.ref_id?.split(':')[0]}` : `/hadiths/${note.ref_id}`} class="note-ref">
          {refLabel}
        </a>
      {/if}
      <Badge text={sourceLabel} variant="default" />
      {#if note.ref_type === 'topic' && note.refs.length > 0}
        <span class="ref-count">{note.refs.length} refs</span>
      {/if}
    </div>
    <div class="note-actions">
      <span class="note-time">{timeAgo}</span>
      {#if onedit}
        <button class="action-btn" onclick={() => onedit?.(note)} aria-label="Edit">&#9998;</button>
      {/if}
      {#if ondelete}
        <button class="action-btn delete-btn" onclick={() => ondelete?.(note)} aria-label="Delete">&times;</button>
      {/if}
    </div>
  </div>

  {#if note.content}
    <div class="note-content">
      {#each contentParts as part}
        {#if part.type === 'text'}
          <span class="text-segment">{part.value}</span>
        {:else if part.type === 'html'}
          {@html part.value}
        {:else if part.type === 'narrator'}
          <MentionChip refType="narrator" refId={part.refId} />
        {:else if part.type === 'ayah'}
          <EmbeddedRef refType="ayah" refId={part.refId} />
        {:else if part.type === 'hadith'}
          <EmbeddedRef refType="hadith" refId={part.refId} />
        {:else if part.type === 'url'}
          <LinkPreviewCard url={part.value} />
        {/if}
      {/each}
    </div>
  {/if}

  {#if note.tags.length > 0}
    <div class="note-tags">
      {#each note.tags as tag}
        <Badge text={tag} variant="default" />
      {/each}
    </div>
  {/if}
</div>

<style>
  .note-card {
    background: var(--note-card-bg);
    border-color: var(--border-subtle);
  }
  .note-card:hover .action-btn {
    opacity: 1;
  }
  .note-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }
  .note-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-wrap: wrap;
  }
  .note-title {
    font-family: var(--font-serif);
    font-weight: var(--font-weight-bold);
    font-size: var(--text-lg);
    color: var(--text-primary);
    line-height: var(--leading-tight);
    letter-spacing: -0.01em;
  }
  .note-ref {
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    color: var(--accent);
    text-decoration: none;
    font-weight: var(--font-weight-semibold);
  }
  .note-ref:hover { text-decoration: underline; color: var(--accent-hover); }
  .ref-count {
    font-size: var(--text-2xs);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  .note-actions {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    flex-shrink: 0;
  }
  .note-time {
    font-size: var(--text-2xs);
    color: var(--text-muted);
  }
  .action-btn {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: var(--text-base);
    padding: var(--space-1) 6px;
    border-radius: var(--radius-sm);
    opacity: 0;
    transition: opacity var(--transition), color var(--transition), background var(--transition);
  }
  .action-btn:hover { color: var(--text-primary); background: var(--bg-hover); }
  .delete-btn:hover { color: var(--error); background: rgba(220, 38, 38, 0.08); }
  .note-content {
    font-family: var(--font-serif);
    font-size: var(--text-base);
    line-height: 1.7;
    color: var(--text-secondary);
    word-break: break-word;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .text-segment {
    white-space: pre-wrap;
  }
  .note-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: var(--space-3);
  }
</style>
