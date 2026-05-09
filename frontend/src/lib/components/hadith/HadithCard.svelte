<script lang="ts">
  import type { ApiHadith, SharhPageRef } from '$lib/types';
  import Button from '$lib/components/common/Button.svelte';
  import HadithBody from '$lib/components/hadith/HadithBody.svelte';
  import { language } from '$lib/stores/language';
  import { proseArabicFontSize } from '$lib/stores/preferences';

  let { hadith, sharhPage, onopensharh, view = 'list' }: {
    hadith: ApiHadith;
    sharhPage?: SharhPageRef;
    onopensharh?: (info: { bookId: number; pageIndex: number; bookName: string; hadithNumber: number }) => void;
    view?: 'list' | 'grid';
  } = $props();

  const bookLabel = $derived(hadith.book_name ?? `Book ${hadith.collection_id}`);
</script>

<article class="hadith-card hadith-card--{view}">
  <a href="/hadiths/{hadith.id}" class="card-link">
    <div class="title-row">
      <h3 class="title">
        {bookLabel} <span class="num mono">#{hadith.hadith_number}</span>
      </h3>
      {#if hadith.narrator_text && view === 'list'}
        <bdi class="narrator">{hadith.narrator_text}</bdi>
      {/if}
    </div>

    {#if hadith.narrator_text && view === 'grid'}
      <p class="narrator narrator--stacked">{hadith.narrator_text}</p>
    {/if}

    <div class="body">
      <HadithBody
        textAr={hadith.text_ar}
        textEn={hadith.text_en}
        language={$language}
        arabicSize={Math.min(view === 'grid' ? 1.05 : 1.2, $proseArabicFontSize)}
        englishSize={view === 'grid' ? 0.95 : 1}
        preview
        previewLength={view === 'grid' ? 110 : 180}
      />
    </div>
  </a>

  {#if sharhPage && onopensharh}
    {@const sp = sharhPage}
    <div class="card-actions">
      <Button
        variant="ghost"
        size="sm"
        onclick={() => onopensharh({ bookId: sp.book_id, pageIndex: sp.page_index, bookName: sp.book_name, hadithNumber: hadith.hadith_number })}
        title="View {sp.book_name}"
      >
        Sharḥ →
      </Button>
    </div>
  {/if}
</article>

<style>
  /* ── Shared ── */
  .title-row {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-3);
    flex-wrap: wrap;
  }
  .title {
    margin: 0;
    font-family: var(--font-serif);
    font-size: var(--text-body-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    line-height: 1.3;
  }
  .num {
    color: var(--text-muted);
    font-weight: var(--font-weight-medium);
  }
  .narrator {
    font-family: var(--font-serif);
    font-size: var(--text-meta);
    color: var(--text-secondary);
    font-style: italic;
    line-height: 1.4;
    text-align: right;
    flex: 1 1 auto;
    min-width: 0;
  }

  /* ── List view ── */
  .hadith-card--list {
    padding: var(--space-5) var(--space-3);
    border-bottom: 1px solid var(--border-subtle);
    transition: background var(--transition);
  }
  .hadith-card--list:hover { background: var(--bg-hover); }
  .hadith-card--list:last-child { border-bottom: none; }

  .hadith-card--list .card-link {
    display: block;
    color: inherit;
    text-decoration: none;
  }
  .hadith-card--list .body { margin-top: var(--space-3); }
  .hadith-card--list .card-actions {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2) 0 0;
  }

  /* ── Grid view ── */
  .hadith-card--grid {
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    padding: var(--space-4);
    transition: border-color var(--transition), background var(--transition);
    height: 100%;
  }
  .hadith-card--grid:hover {
    background: var(--bg-hover);
    border-color: var(--border);
  }
  .hadith-card--grid .card-link {
    display: flex;
    flex-direction: column;
    color: inherit;
    text-decoration: none;
    flex: 1 1 auto;
  }
  .hadith-card--grid .narrator--stacked {
    margin: var(--space-2) 0 0;
    text-align: left;
  }
  .hadith-card--grid .body {
    margin-top: var(--space-2);
    flex: 1 1 auto;
  }
  .hadith-card--grid .card-actions {
    margin-top: var(--space-3);
    padding-top: var(--space-3);
    border-top: 1px solid var(--border-subtle);
    display: flex;
    gap: var(--space-2);
  }

  @media (max-width: 640px) {
    .hadith-card--list .title-row { flex-direction: column; align-items: flex-start; gap: var(--space-1); }
    .hadith-card--list .narrator { text-align: left; }
  }
</style>
