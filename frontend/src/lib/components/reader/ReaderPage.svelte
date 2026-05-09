<script lang="ts">
  import type { BookPage } from '$lib/types';
  import { convertPageToHtml } from '$lib/utils';
  import { proseArabicFontSize } from '$lib/stores/preferences';

  let { page }: { page: BookPage } = $props();

  let html = $derived(convertPageToHtml(page.text));
</script>

<article class="reader-page" dir="rtl" style="font-size: {$proseArabicFontSize}rem">
  {@html html}
  <p class="page-label">{page.vol} / {page.page_num}</p>
</article>

<style>
  .reader-page {
    font-family: var(--font-arabic-text);
    line-height: 2;
    color: var(--text-primary);
    padding: 1.75rem 0 1.25rem;
    border-bottom: 1px solid var(--border-subtle);
    text-align: right;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .reader-page :global(span[data-type="title"]) {
    display: block;
    font-size: 1.2em;
    font-weight: 700;
    text-align: center;
    margin: 0.5rem 0;
    line-height: 1.6;
    color: var(--text-primary);
  }

  .reader-page :global(.block) {
    margin: 0;
  }

  .reader-page :global(.footnotes) {
    font-size: 0.7em;
    color: var(--text-muted);
    margin-top: 1rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--border-subtle);
    line-height: 1.7;
  }

  .reader-page :global(span) {
    color: var(--text-primary);
  }

  .page-label {
    text-align: center;
    font-family: var(--font-sans);
    font-size: 0.8rem;
    color: var(--text-muted);
    margin-top: 1rem;
    direction: ltr;
  }

  @media (max-width: 640px) {
    .reader-page {
      line-height: 1.9;
      padding: 1.25rem 0 1rem;
      gap: 0.75rem;
    }
  }
</style>
