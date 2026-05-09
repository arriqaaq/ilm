<script lang="ts">
  import type { HadithGrading } from '$lib/types';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';

  let { gradings = [] }: { gradings?: HadithGrading[] } = $props();

  function chipVariant(g: HadithGrading): 'success' | 'accent' | 'warning' | 'default' {
    switch (g.grade_normalized) {
      case 'sahih':
        return 'success';
      case 'hasan':
        return 'accent';
      case 'daif':
      case 'mawdu':
        return 'warning';
      default:
        return 'default';
    }
  }

  function sourceHref(g: HadithGrading): string | null {
    if (g.source_book_id == null) return null;
    const page = g.source_page_index ?? 0;
    return `/books/${g.source_book_id}?page=${page}`;
  }
</script>

{#if gradings.length > 0}
  <section class="grading-panel">
    <div class="panel-eyebrow"><Eyebrow>Scholar Rulings</Eyebrow></div>
    <ul class="grading-list">
      {#each gradings as g}
        <li class="grading-row">
          <span class="scholar" lang="ar" dir="rtl">{g.scholar_ar}</span>
          <span class="chip {chipVariant(g)}" lang="ar" dir="rtl">{g.grade}</span>
          {#if g.notes}
            <span class="notes">{g.notes}</span>
          {/if}
          {#if sourceHref(g)}
            <a class="source-link" href={sourceHref(g)}>Source ↗</a>
          {/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .grading-panel {
    margin-top: 0;
    padding: var(--space-4);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    background: var(--bg-surface);
  }
  .panel-eyebrow { margin-bottom: var(--space-3); }

  .grading-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .grading-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) 0;
    border-bottom: 1px solid var(--border-subtle);
    flex-wrap: wrap;
  }
  .grading-row:last-child { border-bottom: none; }

  .scholar {
    font-family: var(--font-arabic);
    font-weight: var(--font-weight-semibold);
    min-width: 6ch;
    font-size: 1.05rem;
    color: var(--text-primary);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    padding: 2px var(--space-3);
    border-radius: var(--radius-pill);
    font-family: var(--font-arabic);
    font-size: var(--text-base);
    font-weight: var(--font-weight-semibold);
    line-height: 1.6;
  }

  .chip.success { background: rgba(21, 128, 61, 0.10); color: var(--success); }
  .chip.accent  { background: var(--accent-muted); color: var(--accent); }
  .chip.warning { background: rgba(180, 83, 9, 0.10); color: var(--warning); }
  .chip.default {
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .notes {
    font-size: var(--text-meta);
    color: var(--text-muted);
    font-style: italic;
  }

  .source-link {
    margin-left: auto;
    font-size: var(--text-meta);
    color: var(--accent);
    text-decoration: none;
  }
  .source-link:hover { text-decoration: underline; }
</style>
