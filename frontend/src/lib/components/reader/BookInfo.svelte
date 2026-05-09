<script lang="ts">
  import type { Book } from '$lib/types';
  import PageHero from '$lib/components/layout/PageHero.svelte';

  let { book }: { book: Book } = $props();
</script>

<PageHero nameEn={book.name_en} nameAr={book.name_ar} arabicFont="prose">
  {#snippet accent()}
    {#if book.author_ar}
      <span class="author arabic-prose" dir="rtl">{book.author_ar}</span>
    {/if}
  {/snippet}
  {#snippet meta()}
    <span class="hero-meta-item"><span class="hero-meta-label">Pages</span> {book.total_pages.toLocaleString()}</span>
    <span class="hero-dot">·</span>
    <span class="hero-meta-item"><span class="hero-meta-label">ID</span> <span class="mono">#{book.book_id}</span></span>
  {/snippet}
</PageHero>

<style>
  .author { font-weight: 500; }
  .hero-meta-item {
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-secondary);
  }
  .hero-meta-label {
    color: var(--text-muted);
    font-size: var(--text-eyebrow);
    text-transform: uppercase;
    letter-spacing: var(--tracking-eyebrow);
    font-weight: var(--font-weight-semibold);
    margin-right: 4px;
  }
  .hero-dot { color: var(--text-muted); }
  .mono { font-family: var(--font-mono); }
</style>
