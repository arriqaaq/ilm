<script lang="ts">
  import { stripHtml } from '$lib/utils';

  let { textAr, textEn, language, arabicSize = 1.5, englishSize = 1.125, preview = false, previewLength = 200 }: {
    textAr?: string | null;
    textEn?: string | null;
    language: 'ar' | 'en';
    arabicSize?: number;
    englishSize?: number;
    preview?: boolean;
    previewLength?: number;
  } = $props();

  function highlightMatn(text: string): string {
    return text
      .replace(/"([^"]+)"/g, '<span class="matn">"$1"</span>')
      .replace(/«([^»]+)»/g, '<span class="matn">«$1»</span>');
  }

  function clip(text: string, n: number): string {
    if (text.length <= n) return text;
    return text.slice(0, n).replace(/\s+\S*$/, '') + '…';
  }

  let chosen = $derived.by(() => {
    if (language === 'en') {
      if (textEn) return { mode: 'en' as const, value: stripHtml(textEn) };
      if (textAr) return { mode: 'ar' as const, value: textAr };
    } else {
      if (textAr) return { mode: 'ar' as const, value: textAr };
      if (textEn) return { mode: 'en' as const, value: stripHtml(textEn) };
    }
    return null;
  });
</script>

{#if chosen}
  {#if chosen.mode === 'ar'}
    <p class="prose-ar arabic-prose" dir="rtl" style="font-size: {arabicSize}rem">
      {#if preview}
        {clip(chosen.value, previewLength)}
      {:else}
        {@html highlightMatn(chosen.value)}
      {/if}
    </p>
  {:else}
    <p class="prose-en" style="font-size: {englishSize}rem">
      {preview ? clip(chosen.value, previewLength) : chosen.value}
    </p>
  {/if}
{/if}

<style>
  .prose-ar {
    color: var(--text-primary);
    line-height: 2;
    margin: 0;
  }
  .prose-ar :global(.matn) {
    color: var(--text-primary);
    font-weight: var(--font-weight-semibold);
  }
  .prose-en {
    font-family: var(--font-serif);
    color: var(--text-primary);
    line-height: 1.7;
    letter-spacing: 0.005em;
    margin: 0;
  }
</style>
