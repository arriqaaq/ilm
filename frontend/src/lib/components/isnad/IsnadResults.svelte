<script lang="ts">
  import type { IsnadSearchResponse, ApiNarratorSearchResult } from '$lib/types';
  import { truncate, stripHtml } from '$lib/utils';
  import { language } from '$lib/stores/language';
  import Badge from '$lib/components/common/Badge.svelte';

  let { result, selectedNarrators }: {
    result: IsnadSearchResponse;
    selectedNarrators: ApiNarratorSearchResult[];
  } = $props();
</script>

<section class="isnad-results">
  <h2>Results ({result.total} hadiths)</h2>
  {#if result.hadiths.length === 0}
    <div class="empty">No hadiths found matching all selected narrators.</div>
  {:else}
    <div class="results-list">
      {#each result.hadiths as h}
        <a href="/hadiths/{h.id}" class="result-card">
          <div class="result-header">
            <Badge text="Book {h.collection_id}" />
            <span class="hadith-num mono">#{h.hadith_number}</span>
          </div>
          {#if h.narrator_text}<p class="narrator">{h.narrator_text}</p>{/if}
          <p class="text">
            {$language === 'en' && h.text_en ? truncate(stripHtml(h.text_en), 200) : truncate(h.text_ar || stripHtml(h.text_en ?? ''), 200)}
          </p>
        </a>
      {/each}
    </div>
  {/if}
</section>

<style>
  .isnad-results h2 { margin-bottom: 12px; }
  .results-list { display: flex; flex-direction: column; gap: 10px; }
  .result-card { display: block; padding: 14px 16px; background: var(--bg-surface); border: 1px solid var(--border); border-radius: var(--radius); color: var(--text-primary); transition: all var(--transition); }
  .result-card:hover { border-color: var(--accent); background: var(--bg-hover); color: var(--text-primary); }
  .result-header { display: flex; align-items: center; gap: 10px; margin-bottom: 6px; }
  .hadith-num { color: var(--text-muted); font-size: 0.8rem; }
  .narrator { color: var(--accent); font-size: 0.85rem; margin-bottom: 4px; }
  .text { color: var(--text-secondary); font-size: 0.85rem; line-height: 1.5; }
  .empty { text-align: center; color: var(--text-muted); padding: 40px; }
</style>
