<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    nameEn,
    nameAr,
    arabicFont = 'prose',
    accent,
    eyebrow,
    meta,
    actions,
  }: {
    nameEn?: string;
    nameAr?: string;
    arabicFont?: 'prose' | 'quran' | 'mono';
    accent?: Snippet;
    eyebrow?: Snippet;
    meta?: Snippet;
    actions?: Snippet;
  } = $props();
</script>

<header class="page-hero">
  {#if eyebrow}<div class="eyebrow-row">{@render eyebrow()}</div>{/if}
  <div class="title-row">
    {#if nameEn}
      <div class="title-en"><h1>{nameEn}</h1></div>
    {/if}
    {#if nameAr}
      <div class="title-ar" dir="rtl">
        <h1 class="ar ar-{arabicFont}">{nameAr}</h1>
      </div>
    {/if}
  </div>
  {#if accent}<div class="accent-row">{@render accent()}</div>{/if}
  {#if meta}<div class="meta-row">{@render meta()}</div>{/if}
  {#if actions}<div class="actions-row">{@render actions()}</div>{/if}
</header>

<style>
  .page-hero {
    border-bottom: 1px solid var(--border-subtle);
    padding: var(--space-7) 0 var(--space-5);
    margin-bottom: var(--space-5);
  }
  .eyebrow-row { margin-bottom: var(--space-3); }

  .title-row {
    display: flex;
    gap: var(--space-6);
    align-items: flex-start;
    justify-content: space-between;
    flex-wrap: wrap;
  }
  .title-en, .title-ar {
    flex: 1 1 auto;
    min-width: 0;
  }
  .title-ar { text-align: right; }
  .title-en h1 {
    margin: 0;
    font-family: var(--font-serif);
    font-size: clamp(2rem, 4.2vw, 3rem);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-tight);
    line-height: 1.15;
    color: var(--text-primary);
  }
  .ar {
    margin: 0;
    font-weight: 700;
    line-height: 1.4;
    color: var(--text-primary);
    font-size: clamp(2.1rem, 4.4vw, 3.2rem);
  }
  .ar-prose { font-family: var(--font-arabic); }
  .ar-quran { font-family: var(--font-quran); }
  .ar-mono  { font-family: var(--font-mono); }

  .accent-row {
    margin-top: var(--space-3);
    color: var(--accent);
    font-family: var(--font-serif);
    font-size: var(--text-base);
    font-weight: 500;
  }
  .meta-row {
    margin-top: var(--space-4);
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }
  .actions-row {
    margin-top: var(--space-4);
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
  }

  @media (max-width: 640px) {
    .page-hero { padding: var(--space-5) 0 var(--space-4); }
    .title-row { flex-direction: column; gap: var(--space-3); }
    .title-en h1, .ar { font-size: 1.75rem; }
    .title-ar { text-align: left; }
    .accent-row { text-align: left; }
  }
</style>
