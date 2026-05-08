<script lang="ts">
  import type { ApiHadith, SharhPageRef } from '$lib/types';
  import { truncate, stripHtml } from '$lib/utils';
  import Badge from '$lib/components/common/Badge.svelte';
  import { language } from '$lib/stores/language';

  let { hadith, sharhPage, onopensharh }: {
    hadith: ApiHadith;
    sharhPage?: SharhPageRef;
    onopensharh?: (info: { bookId: number; pageIndex: number; bookName: string; hadithNumber: number }) => void;
  } = $props();
</script>

<div class="card card-stripe hadith-card-wrapper">
  <a href="/hadiths/{hadith.id}" class="card-link hadith-card">
    <div class="card-header">
      {#if hadith.book_name}
        <Badge text={hadith.book_name} variant="accent" />
      {:else}
        <Badge text="Book {hadith.collection_id}" />
      {/if}
      <span class="hadith-num mono">#{hadith.hadith_number}</span>
    </div>

    {#if hadith.narrator_text}
      <p class="narrator">{hadith.narrator_text}</p>
    {/if}

    {#if $language === 'en' && hadith.text_en}
      <p class="text-preview">{truncate(stripHtml(hadith.text_en), 180)}</p>
    {:else if hadith.text_ar}
      <p class="text-ar arabic-text" dir="rtl">{truncate(hadith.text_ar, 150)}</p>
    {:else if hadith.text_en}
      <p class="text-preview">{truncate(stripHtml(hadith.text_en), 180)}</p>
    {/if}
  </a>

  {#if sharhPage && onopensharh}
    <div class="card-actions">
      <button
        class="btn btn-secondary btn-sm"
        onclick={() => onopensharh({ bookId: sharhPage.book_id, pageIndex: sharhPage.page_index, bookName: sharhPage.book_name, hadithNumber: hadith.hadith_number })}
        title="View {sharhPage.book_name}"
      >
        Sharh
      </button>
    </div>
  {/if}
</div>

<style>
  .hadith-card-wrapper {
    overflow: hidden;
    padding: 0;
  }
  .hadith-card-wrapper:hover {
    background: var(--bg-hover);
  }

  .hadith-card {
    padding: var(--space-card-y) var(--space-card-x);
  }

  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-2);
  }

  .hadith-num {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .narrator {
    color: var(--accent);
    font-size: var(--text-sm);
    margin-bottom: var(--space-2);
    font-weight: var(--font-weight-medium);
  }

  .text-preview {
    font-family: var(--font-serif);
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
    margin-bottom: var(--space-2);
  }

  .text-ar {
    color: var(--text-secondary);
    font-size: var(--text-md);
    opacity: 0.9;
  }

  .card-actions {
    padding: 0 var(--space-card-x) var(--space-3);
    display: flex;
    gap: var(--space-2);
  }
</style>
