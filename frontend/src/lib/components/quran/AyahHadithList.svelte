<script lang="ts">
  import type { AyahHadithResponse } from '$lib/types';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import HadithBody from '$lib/components/hadith/HadithBody.svelte';
  import { language } from '$lib/stores/language';
  import { proseArabicFontSize } from '$lib/stores/preferences';

  let { data }: { data: AyahHadithResponse } = $props();
</script>

{#if data.curated.length > 0}
  <div class="section">
    <div class="section-eyebrow"><Eyebrow>Referenced Hadiths</Eyebrow></div>
    {#each data.curated as hadith}
      <div class="hadith-item">
        <div class="hadith-meta">
          <span class="book-name">{hadith.book_name ?? 'Unknown'}</span>
          <span class="dot" aria-hidden="true">·</span>
          <span class="hadith-num mono">#{hadith.hadith_number}</span>
          {#if hadith.grade}
            <span class="grade-badge">{hadith.grade}</span>
          {/if}
          <a href="/hadiths/{hadith.id}" class="detail-link">View →</a>
        </div>
        <HadithBody
          textAr={hadith.matn ?? null}
          textEn={hadith.text_en ?? null}
          language={$language}
          arabicSize={Math.min(1.1, $proseArabicFontSize)}
          englishSize={0.95}
          preview
          previewLength={300}
        />
      </div>
    {/each}
  </div>
{/if}

{#if data.related && data.related.length > 0}
  <div class="section">
    <div class="section-eyebrow"><Eyebrow tone="muted">Related Hadiths</Eyebrow></div>
    {#each data.related as hadith}
      <div class="hadith-item">
        <div class="hadith-meta">
          <span class="hadith-num mono">#{hadith.hadith_number}</span>
          {#if hadith.score}
            <span class="score mono">{hadith.score.toFixed(3)}</span>
          {/if}
          <a href="/hadiths/{hadith.id}" class="detail-link">View →</a>
        </div>
        {#if hadith.text_en}
          <HadithBody
            textAr={null}
            textEn={hadith.text_en}
            language="en"
            englishSize={0.95}
            preview
            previewLength={200}
          />
        {/if}
      </div>
    {/each}
  </div>
{/if}

{#if data.curated.length === 0 && (!data.related || data.related.length === 0)}
  <div class="empty">No hadiths found for this verse.</div>
{/if}

<style>
  .section { margin-bottom: var(--space-4); }
  .section:last-child { margin-bottom: 0; }
  .section-eyebrow { margin-bottom: var(--space-2); }

  .hadith-item {
    padding: var(--space-3) 0;
    border-bottom: 1px solid var(--border-subtle);
  }
  .hadith-item:last-child { border-bottom: none; }

  .hadith-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
    flex-wrap: wrap;
    font-size: var(--text-meta);
  }
  .book-name {
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
  }
  .hadith-num { color: var(--text-muted); }
  .dot { color: var(--text-muted); }
  .grade-badge {
    font-size: var(--text-2xs);
    padding: 1px 6px;
    border-radius: var(--radius-pill);
    background: var(--accent-muted);
    color: var(--accent);
    font-weight: var(--font-weight-semibold);
  }
  .score {
    color: var(--success);
  }
  .detail-link {
    margin-left: auto;
    color: var(--accent);
    font-size: var(--text-meta);
  }
  .detail-link:hover { text-decoration: underline; }
  .empty {
    font-size: var(--text-meta);
    color: var(--text-muted);
  }
</style>
