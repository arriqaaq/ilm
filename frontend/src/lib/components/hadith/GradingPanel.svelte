<script lang="ts">
  import type { HadithGrading } from '$lib/types';

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
    return `/tafsir/${g.source_book_id}?page=${page}`;
  }
</script>

{#if gradings.length > 0}
  <section class="grading-panel">
    <h2>Scholar Rulings</h2>
    <ul class="grading-list">
      {#each gradings as g}
        <li class="grading-row">
          <span class="scholar" lang="ar" dir="rtl">{g.scholar_ar}</span>
          <span class="chip {chipVariant(g)}" lang="ar" dir="rtl">{g.grade}</span>
          {#if g.notes}
            <span class="notes">{g.notes}</span>
          {/if}
          {#if sourceHref(g)}
            <a class="source-link" href={sourceHref(g)}>Open source &#x2197;</a>
          {/if}
        </li>
      {/each}
    </ul>
  </section>
{/if}

<style>
  .grading-panel {
    margin-top: 1.5rem;
  }

  .grading-panel h2 {
    margin: 0 0 0.5rem 0;
    font-size: 1rem;
    color: var(--text-secondary);
    font-weight: 600;
  }

  .grading-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .grading-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg-surface);
    flex-wrap: wrap;
  }

  .scholar {
    font-weight: 600;
    min-width: 7ch;
    color: var(--text-primary);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    padding: 2px 10px;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    font-weight: 500;
  }

  .chip.success {
    background: rgba(74, 222, 128, 0.15);
    color: var(--success);
  }

  .chip.accent {
    background: var(--accent-muted);
    color: var(--accent);
  }

  .chip.warning {
    background: rgba(248, 113, 113, 0.15);
    color: var(--danger, #f87171);
  }

  .chip.default {
    background: var(--bg-surface);
    color: var(--text-secondary);
    border: 1px solid var(--border);
  }

  .notes {
    font-size: 0.8rem;
    color: var(--text-muted);
    font-style: italic;
  }

  .source-link {
    margin-left: auto;
    font-size: 0.85rem;
    color: var(--accent);
    text-decoration: none;
  }

  .source-link:hover {
    text-decoration: underline;
  }
</style>
