<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { getAllTafsirsForAyah, getSurah } from '$lib/api';
  import type { AllTafsirsResponse, ApiAyah, SurahDetailResponse } from '$lib/types';
  import { parseVerseRef, AYAH_COUNTS } from '$lib/constants/ayahCounts';
  import VersePicker from '$lib/components/tafsir/VersePicker.svelte';
  import TafsirAccordion from '$lib/components/tafsir/TafsirAccordion.svelte';
  import TafsirAskDrawer from '$lib/components/tafsir/TafsirAskDrawer.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import PageHeader from '$lib/components/common/PageHeader.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import Button from '$lib/components/common/Button.svelte';
  import { appConfig } from '$lib/stores/config';
  import { preferences } from '$lib/stores/preferences';

  let current = $derived.by(() => {
    const raw = page.url.searchParams.get('verse') ?? '1:1';
    return parseVerseRef(raw) ?? { surah: 1, ayah: 1 };
  });
  const surah = $derived(current.surah);
  const ayah = $derived(current.ayah);

  let surahData: SurahDetailResponse | null = $state(null);
  let surahLoading = $state(false);
  let tafsirData: AllTafsirsResponse | null = $state(null);
  let tafsirLoading = $state(false);
  let errorMsg: string | null = $state(null);

  let askOpen = $state(false);
  let lastLoadedSurah = $state(-1);

  $effect(() => {
    if (surah === lastLoadedSurah && surahData) return;
    surahLoading = true;
    getSurah(surah)
      .then((d) => { surahData = d; lastLoadedSurah = surah; })
      .catch((e) => { errorMsg = `Failed to load surah: ${e?.message ?? e}`; })
      .finally(() => { surahLoading = false; });
  });

  $effect(() => {
    tafsirLoading = true;
    errorMsg = null;
    // Anchor on (surah, ayah) so the fetch re-runs whenever either changes.
    const s = surah;
    const a = ayah;
    getAllTafsirsForAyah(s, a)
      .then((d) => { tafsirData = d; })
      .catch((e) => {
        tafsirData = null;
        errorMsg = `Failed to load tafsir: ${e?.message ?? e}`;
      })
      .finally(() => { tafsirLoading = false; });
  });

  const currentAyah = $derived.by(() => {
    if (!surahData) return null;
    return surahData.ayahs.find((row: ApiAyah) => row.ayah_number === ayah) ?? null;
  });
  const surahName = $derived.by(() =>
    surahData ? surahData.surah.name_translit : `Surah ${surah}`
  );
  const surahNameAr = $derived.by(() => (surahData ? surahData.surah.name_ar : ''));

  function handlePick(v: { surah: number; ayah: number }) {
    const url = new URL(page.url);
    url.searchParams.set('verse', `${v.surah}:${v.ayah}`);
    goto(url.pathname + url.search, { keepFocus: true, noScroll: true, replaceState: false });
  }

  function handleKey(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
      e.preventDefault();
      askOpen = !askOpen;
    }
  }
</script>

<svelte:window onkeydown={handleKey} />

<svelte:head>
  <title>Tafsir · {surah}:{ayah}</title>
</svelte:head>

<div class="page-shell-narrow">
  <div class="header-row">
    <PageHeader
      eyebrow="Tafsīr"
      title="Qurʾānic Exegesis"
      subtitle="Pick a verse to read every tafsīr for it, or ask a free-form question across the corpus."
    />
    {#if $appConfig.advanced_enabled}
      <Button variant="primary" size="md" onclick={() => (askOpen = true)} title="Ask AI (⌘K)">
        Ask AI <kbd class="kbd">⌘K</kbd>
      </Button>
    {/if}
  </div>

  <VersePicker surah={surah} ayah={ayah} onsubmit={handlePick} />

  <section class="ayah-context">
    <div class="context-meta">
      <Eyebrow>{surahName} · {surahNameAr}</Eyebrow>
      <span class="ref-label mono">{surah}:{ayah}</span>
    </div>
    {#if currentAyah}
      <p class="ayah-ar arabic-prose" dir="rtl" style="font-size: {$preferences.arabicFontSize}rem">{currentAyah.text_ar}</p>
      {#if currentAyah.text_en}
        <p class="ayah-en" style="font-size: {$preferences.englishFontSize}rem">{currentAyah.text_en}</p>
      {/if}
    {:else if surahLoading}
      <div class="context-placeholder"><LoadingSpinner /></div>
    {:else}
      <p class="context-placeholder">Ayah text unavailable.</p>
    {/if}
  </section>

  <section class="tafsir-body">
    {#if tafsirLoading && !tafsirData}
      <div class="loading-row"><LoadingSpinner /> <span>Loading tafsīr…</span></div>
    {:else if errorMsg}
      <div class="error-state">{errorMsg}</div>
    {:else if tafsirData}
      <TafsirAccordion entries={tafsirData.entries} english={tafsirData.english} />
    {/if}
  </section>
</div>

{#if $appConfig.advanced_enabled}
  <TafsirAskDrawer
    open={askOpen}
    verse={{ surah, ayah }}
    onclose={() => (askOpen = false)}
  />
{/if}

<style>
  .header-row {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: var(--space-4);
    margin-bottom: var(--space-6);
  }
  .header-row :global(.page-header) { margin-bottom: 0; flex: 1; }
  .kbd {
    background: rgba(255, 255, 255, 0.18);
    padding: 1px var(--space-2);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    margin-left: var(--space-2);
  }

  .ayah-context {
    margin: var(--space-6) 0;
    padding: var(--space-5);
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
  }
  .context-meta {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: var(--space-4);
  }
  .ref-label {
    color: var(--accent);
    background: var(--accent-muted);
    padding: 2px var(--space-2);
    border-radius: var(--radius-pill);
    font-size: var(--text-meta);
  }
  .ayah-ar {
    line-height: 2.2;
    color: var(--text-primary);
    margin: 0;
  }
  .ayah-en {
    margin: var(--space-3) 0 0;
    font-family: var(--font-serif);
    line-height: 1.7;
    color: var(--text-secondary);
  }
  .context-placeholder {
    color: var(--text-muted);
    font-size: var(--text-meta);
    text-align: center;
  }

  .tafsir-body { min-height: 120px; }
  .loading-row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-8) var(--space-5);
    color: var(--text-muted);
    font-size: var(--text-meta);
  }
  .error-state {
    padding: var(--space-6) var(--space-5);
    color: var(--error);
    font-size: var(--text-meta);
  }
</style>
