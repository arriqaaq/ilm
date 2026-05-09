<script lang="ts">
  import { getSurah, getHadith, getNarrator } from '$lib/api';
  import { language } from '$lib/stores/language';
  import { stripHtml } from '$lib/utils';
  import { preferences, proseArabicFontSize } from '$lib/stores/preferences';

  let { refType, refId }: {
    refType: 'ayah' | 'hadith' | 'narrator';
    refId: string;
  } = $props();

  let textAr = $state('');
  let textEn = $state('');
  let label = $state('');
  let narratorName = $state('');
  let bookName = $state('');
  let hadithNumber = $state('');
  let chapterName = $state('');
  let loading = $state(true);
  let failed = $state(false);

  let href = $derived(
    refType === 'ayah'
      ? `/quran/${refId.split(':')[0]}`
      : refType === 'hadith'
        ? `/hadiths/${encodeURIComponent(refId)}`
        : `/narrators/${encodeURIComponent(refId)}`
  );

  function decodeEntities(text: string): string {
    return text
      .replace(/&quot;/g, '"')
      .replace(/&amp;/g, '&')
      .replace(/&lt;/g, '<')
      .replace(/&gt;/g, '>')
      .replace(/&#39;/g, "'")
      .replace(/&#x27;/g, "'");
  }

  function truncate(text: string, max: number): string {
    if (text.length <= max) return text;
    return text.slice(0, max).trimEnd() + '…';
  }

  $effect(() => {
    loading = true;
    failed = false;
    textAr = '';
    textEn = '';
    narratorName = '';
    bookName = '';
    hadithNumber = '';
    chapterName = '';

    if (refType === 'ayah') {
      const [s, a] = refId.split(':').map(Number);
      label = `${s}:${a}`;
      getSurah(s).then(res => {
        const ayah = res.ayahs.find((ay: any) => ay.ayah_number === a);
        if (ayah) {
          textAr = ayah.text_ar ?? '';
          textEn = ayah.text_en ?? '';
          bookName = res.surah?.name_en ?? '';
        } else {
          failed = true;
        }
      }).catch(() => { failed = true; })
        .finally(() => { loading = false; });
    } else if (refType === 'hadith') {
      label = refId;
      getHadith(refId).then(res => {
        const h = res.hadith;
        const rawAr = h.matn ?? h.text_ar ?? '';
        const rawEn = h.text_en ?? '';
        textAr = decodeEntities(stripHtml(rawAr));
        textEn = decodeEntities(stripHtml(rawEn));
        narratorName = h.narrator_text ?? '';
        bookName = h.book_name ?? 'Hadith';
        hadithNumber = String(h.hadith_number ?? '');
        chapterName = h.chapter_name ?? '';
        label = `${bookName} ${hadithNumber}`;
      }).catch(() => { failed = true; })
        .finally(() => { loading = false; });
    } else if (refType === 'narrator') {
      label = refId;
      getNarrator(refId).then(res => {
        label = res.narrator.name_en ?? res.narrator.name_ar ?? refId;
        textEn = res.narrator.bio ?? '';
      }).catch(() => { failed = true; })
        .finally(() => { loading = false; });
    } else {
      label = refId;
      loading = false;
    }
  });
</script>

{#if refType === 'hadith'}
  <a {href} class="hadith-card" draggable="false">
    {#if loading}
      <div class="card-loading">Loading hadith...</div>
    {:else if failed}
      <div class="card-failed">Could not load hadith</div>
    {:else}
      {#if narratorName}
        <div class="hadith-narrator">{narratorName}</div>
      {/if}
      <div class="hadith-body">
        {#if textEn}
          <div class="hadith-en" style="font-size: {$preferences.englishFontSize}rem">{truncate(textEn, 400)}</div>
        {/if}
        {#if textAr}
          <div class="hadith-ar" dir="rtl" style="font-size: {$proseArabicFontSize}rem">{truncate(textAr, 500)}</div>
        {/if}
      </div>
      <div class="hadith-footer">
        <span class="footer-item"><strong>Reference</strong>: {bookName} {hadithNumber}</span>
        {#if chapterName}
          <span class="footer-item"><strong>In-book reference</strong>: {chapterName}</span>
        {/if}
      </div>
    {/if}
  </a>

{:else if refType === 'ayah'}
  <a {href} class="ayah-card" draggable="false">
    {#if loading}
      <div class="card-loading">Loading ayah...</div>
    {:else if failed}
      <div class="card-failed">Could not load ayah</div>
    {:else}
      <div class="ayah-badge">{label}</div>
      {#if textAr}
        <div class="ayah-ar" dir="rtl" style="font-size: {$preferences.arabicFontSize}rem">{textAr}</div>
      {/if}
      {#if textEn}
        <div class="ayah-en" style="font-size: {$preferences.englishFontSize}rem">{textEn}</div>
      {/if}
    {/if}
  </a>

{:else}
  <a {href} class="narrator-card" draggable="false">
    <span class="narrator-badge">Narrator</span>
    <span class="narrator-name">{label}</span>
    {#if textEn}
      <div class="narrator-bio">{truncate(textEn, 150)}</div>
    {/if}
  </a>
{/if}

<style>
  .hadith-card {
    display: block;
    margin: 12px 0;
    padding: 24px 28px;
    background: rgba(var(--hadith-embed-rgb), 0.04);
    border: 1px solid rgba(var(--hadith-embed-rgb), 0.15);
    border-radius: var(--radius-xl);
    text-decoration: none;
    color: inherit;
    transition: all var(--transition);
    box-shadow: 0 1px 3px rgba(0,0,0,0.03);
  }
  .hadith-card:hover {
    background: rgba(var(--hadith-embed-rgb), 0.07);
    border-color: rgba(var(--hadith-embed-rgb), 0.25);
    box-shadow: 0 4px 16px rgba(var(--hadith-embed-rgb), 0.08);
  }
  .hadith-narrator {
    font-family: var(--font-serif);
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--hadith-narrator);
    margin-bottom: 16px;
    line-height: 1.4;
  }
  .hadith-body {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 24px;
    margin-bottom: 16px;
  }
  /* English / Arabic body — font-size set inline via preferences */
  .hadith-en {
    font-family: var(--font-serif);
    line-height: 1.8;
    color: var(--text-secondary);
  }
  .hadith-ar {
    font-family: var(--font-arabic-text);
    line-height: 2.2;
    color: var(--text-primary);
    text-align: right;
  }
  .hadith-footer {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 24px;
    padding-top: 14px;
    border-top: 1px solid rgba(var(--hadith-embed-rgb), 0.1);
  }
  .footer-item {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-family: var(--font-sans);
  }
  .footer-item strong {
    font-weight: 600;
    color: var(--text-secondary);
  }

  .ayah-card {
    display: block;
    margin: 12px 0;
    padding: 24px 28px;
    background: var(--accent-muted);
    border: 1px solid color-mix(in srgb, var(--accent) 20%, transparent);
    border-radius: var(--radius-xl);
    text-decoration: none;
    color: inherit;
    transition: all var(--transition);
    text-align: center;
    position: relative;
    box-shadow: 0 1px 3px rgba(0,0,0,0.03);
  }
  .ayah-card:hover {
    border-color: color-mix(in srgb, var(--accent) 35%, transparent);
    box-shadow: 0 4px 16px rgba(200, 169, 106, 0.1);
  }
  .ayah-badge {
    position: absolute;
    top: 12px;
    right: 16px;
    font-size: 0.7rem;
    font-weight: 700;
    font-family: var(--font-mono);
    color: var(--accent);
    background: var(--bg-primary);
    padding: 3px 10px;
    border-radius: 10px;
    border: 1px solid color-mix(in srgb, var(--accent) 25%, transparent);
  }
  /* Embedded ayah — font-size set inline via preferences */
  .ayah-ar {
    font-family: var(--font-arabic-text);
    line-height: 2.4;
    color: var(--text-primary);
    margin-bottom: 12px;
  }
  .ayah-en {
    font-family: var(--font-serif);
    line-height: 1.7;
    color: var(--text-secondary);
    font-style: italic;
  }

  .narrator-card {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    margin: 4px 0;
    padding: 8px 16px;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    text-decoration: none;
    color: inherit;
    transition: all var(--transition);
    box-shadow: 0 1px 3px rgba(0,0,0,0.03);
  }
  .narrator-card:hover {
    border-color: var(--accent);
    box-shadow: 0 2px 8px rgba(200, 169, 106, 0.1);
  }
  .narrator-badge {
    font-size: 0.6rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 2px 10px;
    border-radius: 10px;
    background: var(--accent-muted);
    color: var(--accent);
  }
  .narrator-name {
    font-family: var(--font-serif);
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--text-primary);
  }
  .narrator-bio {
    width: 100%;
    font-family: var(--font-serif);
    font-size: 0.82rem;
    line-height: 1.6;
    color: var(--text-muted);
    margin-top: 4px;
  }

  .card-loading, .card-failed {
    font-size: 0.8rem;
    color: var(--text-muted);
    font-style: italic;
    font-family: var(--font-serif);
    padding: 8px 0;
  }

  @media (max-width: 640px) {
    .hadith-body {
      grid-template-columns: 1fr;
      gap: 16px;
    }
    .hadith-card, .ayah-card {
      padding: 18px 20px;
    }
  }
</style>
