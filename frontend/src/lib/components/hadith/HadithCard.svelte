<script lang="ts">
  import type { ApiHadith, SharhPageRef } from '$lib/types';
  import Button from '$lib/components/common/Button.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import HadithBody from '$lib/components/hadith/HadithBody.svelte';
  import { language } from '$lib/stores/language';
  import { proseArabicFontSize } from '$lib/stores/preferences';

  let { hadith, sharhPage, onopensharh }: {
    hadith: ApiHadith;
    sharhPage?: SharhPageRef;
    onopensharh?: (info: { bookId: number; pageIndex: number; bookName: string; hadithNumber: number }) => void;
  } = $props();
</script>

<article class="hadith-card-row">
  <a href="/hadiths/{hadith.id}" class="card-link">
    <div class="meta">
      <Eyebrow>
        {hadith.book_name ?? `Book ${hadith.collection_id}`} · #{hadith.hadith_number}
      </Eyebrow>
    </div>
    {#if hadith.narrator_text}
      <p class="narrator">{hadith.narrator_text}</p>
    {/if}
    <div class="body">
      <HadithBody
        textAr={hadith.text_ar}
        textEn={hadith.text_en}
        language={$language}
        arabicSize={Math.min(1.2, $proseArabicFontSize)}
        englishSize={1}
        preview
        previewLength={180}
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
  .hadith-card-row {
    padding: var(--space-5) 0;
    border-bottom: 1px solid var(--border-subtle);
    transition: background var(--transition);
  }
  .hadith-card-row:hover { background: var(--bg-hover); }
  .hadith-card-row:last-child { border-bottom: none; }

  .card-link {
    display: block;
    color: inherit;
    text-decoration: none;
    padding: 0 var(--space-2);
  }
  .meta { margin-bottom: var(--space-2); }

  .narrator {
    font-family: var(--font-serif);
    font-size: var(--text-meta);
    color: var(--text-secondary);
    margin: 0 0 var(--space-2);
    font-style: italic;
  }
  .body { margin-top: var(--space-2); }

  .card-actions {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-2) 0;
  }
</style>
