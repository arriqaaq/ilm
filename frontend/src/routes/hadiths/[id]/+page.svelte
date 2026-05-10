<script lang="ts">
  import { page } from '$app/state';
  import { getHadith, getChainGraph, getHadithSharhPages, getHadithGradings } from '$lib/api';
  import type { HadithDetailResponse, GraphData, SharhPageRef, HadithGrading } from '$lib/types';
  import { language } from '$lib/stores/language';
  import { preferences, proseArabicFontSize } from '$lib/stores/preferences';
  import NarratorChip from '$lib/components/narrator/NarratorChip.svelte';
  import Badge from '$lib/components/common/Badge.svelte';
  import Button from '$lib/components/common/Button.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import SectionHeading from '$lib/components/common/SectionHeading.svelte';
  import Divider from '$lib/components/common/Divider.svelte';
  import PageHero from '$lib/components/layout/PageHero.svelte';
  import TwoColumn from '$lib/components/layout/TwoColumn.svelte';
  import HadithBody from '$lib/components/hadith/HadithBody.svelte';
  import ChainView from '$lib/components/graph/ChainView.svelte';
  import GradingPanel from '$lib/components/hadith/GradingPanel.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import NoteModal from '$lib/components/notes/NoteModal.svelte';
  import SavePopover from '$lib/components/notes/SavePopover.svelte';
  import BookViewerModal from '$lib/components/reader/BookViewerModal.svelte';

  let data: HadithDetailResponse | null = $state(null);
  let graphData: GraphData | null = $state(null);
  let gradings: HadithGrading[] = $state([]);
  let loading = $state(true);
  let showNotePanel = $state(false);
  let showSavePopover = $state(false);
  let saveBtnEl: HTMLSpanElement | undefined = $state();
  let sharhPage: SharhPageRef | null = $state(null);
  let sharhTarget: { bookId: number; pageIndex: number; bookName: string; hadithNumber: number } | null = $state(null);

  let id = $derived(page.params.id);

  $effect(() => {
    if (!id) return;
    loading = true; sharhPage = null; gradings = [];
    Promise.all([getHadith(id), getChainGraph(id), getHadithGradings(id)])
      .then(([d, g, gr]) => {
        data = d; graphData = g; gradings = gr.gradings;
        if (d.hadith.hadith_number && d.hadith.collection_id) {
          getHadithSharhPages(d.hadith.collection_id, [d.hadith.hadith_number])
            .then(res => {
              const ref = res.mappings[String(d.hadith.hadith_number)];
              if (ref) sharhPage = ref;
            }).catch(() => {});
        }
      })
      .catch((e) => console.error('Failed to load hadith:', e))
      .finally(() => { loading = false; });
  });
</script>

<div class="reader">
  {#if loading}
    <div class="reader-loading"><LoadingSpinner /></div>
  {:else if data}
    {@const d = data}
    <article class="reader-column">
      <PageHero
        nameEn={d.hadith.book_name ?? `Collection #${d.hadith.collection_id}`}
        nameAr={`#${d.hadith.hadith_number}`}
        arabicFont="mono"
      >
        {#snippet eyebrow()}
          {#if d.hadith.chapter_name}<Eyebrow>{d.hadith.chapter_name}</Eyebrow>{/if}
        {/snippet}
        {#snippet meta()}
          {#if d.hadith.hadith_type}
            <span class="hero-meta-item"><span class="hero-meta-label">Type</span> {d.hadith.hadith_type}</span>
            <span class="hero-dot">·</span>
          {/if}
          <span class="hero-meta-item"><span class="hero-meta-label">Narrators</span> {d.narrators.length}</span>
          {#if gradings.length > 0}
            <span class="hero-dot">·</span>
            <span class="hero-meta-item"><span class="hero-meta-label">Gradings</span> {gradings.length}</span>
          {/if}
        {/snippet}
        {#snippet actions()}
          <span class="btn-anchor" bind:this={saveBtnEl}>
            <Button variant="ghost" size="sm" onclick={() => { showSavePopover = !showSavePopover; }}>♡ Save</Button>
          </span>
          <Button variant="ghost" size="sm" onclick={() => { showNotePanel = true; }}>✎ Note</Button>
          {#if sharhPage}
            {@const sp = sharhPage}
            <Button
              variant="secondary" size="sm"
              onclick={() => { sharhTarget = { bookId: sp.book_id, pageIndex: sp.page_index, bookName: sp.book_name, hadithNumber: data?.hadith.hadith_number ?? 0 }; }}
            >Sharḥ →</Button>
          {/if}
        {/snippet}
      </PageHero>

      {#if data.hadith.narrator_text}
        <section class="narrator-block">
          <Eyebrow>Narrated by</Eyebrow>
          <p class="narrator-text">{data.hadith.narrator_text}</p>
        </section>
      {/if}

      {#if data.hadith.topics && data.hadith.topics.length > 0}
        <div class="topics">
          {#each data.hadith.topics as topic}
            <Badge text={topic} variant="default" />
          {/each}
        </div>
      {/if}

      <div class="reading-page">
        <HadithBody
          textAr={data.hadith.text_ar}
          textEn={data.hadith.text_en}
          language={$language}
          arabicSize={$proseArabicFontSize}
          englishSize={$preferences.englishFontSize}
        />
      </div>

      <p class="page-label">
        Ḥadīth #{data.hadith.hadith_number}{#if data.hadith.book_name} · {data.hadith.book_name}{/if}
      </p>
    </article>

    {#if data.narrators.length > 0}
      <Divider variant="ornamental" />
      <section class="extra-section column-narrow">
        <SectionHeading eyebrow="Narrators" title="Transmitters" level={2} />
        <div class="chips">
          {#each data.narrators as narrator}<NarratorChip {narrator} />{/each}
        </div>
      </section>
    {/if}

    <Divider variant="ornamental" />
    <div class="column-wide">
      <TwoColumn sidebarWidth={340} sticky>
        {#snippet main()}
          <section class="chain-col">
            <SectionHeading eyebrow="Isnād" title="Narrator Chain" level={2} />
            <ChainView data={graphData} />
          </section>
        {/snippet}
        {#snippet sidebar()}
          <GradingPanel {gradings} />
        {/snippet}
      </TwoColumn>
    </div>

    {#if data.linked_ayahs && data.linked_ayahs.length > 0}
      <Divider variant="ornamental" />
      <section class="extra-section column-narrow">
        <SectionHeading eyebrow="References" title="Quranic Verses" level={2} />
        <div class="ayah-list">
          {#each data.linked_ayahs as ayah}
            <a href="/quran/{ayah.surah_number}" class="ayah-item">
              <div class="ayah-meta"><span class="ayah-ref">{ayah.surah_number}:{ayah.ayah_number}</span></div>
              <div class="ayah-text arabic-prose" dir="rtl" style="font-size: {$preferences.arabicFontSize}rem">{ayah.text_ar}</div>
              {#if ayah.text_en}<div class="ayah-text-en" style="font-size: {$preferences.englishFontSize}rem">{ayah.text_en}</div>{/if}
            </a>
          {/each}
        </div>
      </section>
    {/if}

    {#if data.parallel_hadiths && data.parallel_hadiths.length > 0}
      <Divider variant="ornamental" />
      <section class="extra-section column-narrow">
        <SectionHeading
          eyebrow="Also Recorded In"
          title="Parallel Narrations"
          level={2}
        />
        <p class="parallel-note">
          The same prophetic teaching appears in another collection. No grade is
          inherited — open the parallel to see its own ruling.
        </p>
        <div class="similar-list">
          {#each data.parallel_hadiths as parallel}
            <a href="/hadiths/{parallel.id}" class="similar-item">
              <div class="similar-meta">
                <span class="similar-ref">#{parallel.hadith_number}</span>
                {#if parallel.book_name}<Badge text={parallel.book_name} variant="accent" />{/if}
              </div>
              <HadithBody
                textAr={parallel.text_ar} textEn={parallel.text_en}
                language={$language} arabicSize={$proseArabicFontSize} englishSize={1}
                preview previewLength={150}
              />
            </a>
          {/each}
        </div>
      </section>
    {/if}

    {#if data.similar_hadiths && data.similar_hadiths.length > 0}
      <Divider variant="ornamental" />
      <section class="extra-section column-narrow">
        <SectionHeading eyebrow="Similar" title="Related Hadiths" level={2} />
        <div class="similar-list">
          {#each data.similar_hadiths as similar}
            <a href="/hadiths/{similar.id}" class="similar-item">
              <div class="similar-meta">
                <span class="similar-ref">#{similar.hadith_number}</span>
                {#if similar.book_name}<Badge text={similar.book_name} variant="accent" />{/if}
              </div>
              <HadithBody
                textAr={similar.text_ar} textEn={similar.text_en}
                language={$language} arabicSize={$proseArabicFontSize} englishSize={1}
                preview previewLength={150}
              />
            </a>
          {/each}
        </div>
      </section>
    {/if}
  {:else}
    <div class="reader-empty">Hadith not found.</div>
  {/if}
</div>

{#if showSavePopover && data}
  <SavePopover refType="hadith" refId={data.hadith.id}
    refLabel="Hadith #{data.hadith.hadith_number}"
    anchorX={saveBtnEl ? saveBtnEl.getBoundingClientRect().left : 100}
    anchorY={saveBtnEl ? saveBtnEl.getBoundingClientRect().bottom + 4 : 100}
    onclose={() => { showSavePopover = false; }} />
{/if}
{#if showNotePanel && data}
  <NoteModal refType="hadith" refId={data.hadith.id} refLabel="Hadith #{data.hadith.hadith_number}" onclose={() => { showNotePanel = false; }} />
{/if}
{#if sharhTarget}
  <BookViewerModal bookId={sharhTarget.bookId} pageIndex={sharhTarget.pageIndex}
    title={sharhTarget.bookName} subtitle="Hadith {sharhTarget.hadithNumber}"
    onclose={() => { sharhTarget = null; }} />
{/if}

<style>
  .reader {
    min-height: 100vh;
    background: var(--reader-bg);
    color: var(--reader-fg);
    padding: var(--space-8) 0 var(--space-12);
  }
  .reader-loading,
  .reader-empty {
    max-width: 1024px;
    margin: 0 auto;
    padding: var(--space-12) var(--space-6);
    text-align: center;
    color: var(--text-muted);
  }

  .reader-column {
    max-width: 720px;
    margin: 0 auto;
    padding: 0 var(--space-6);
  }
  .column-narrow { max-width: 720px; margin: 0 auto; padding: 0 var(--space-6); }
  .column-wide   { max-width: 1024px; margin: 0 auto; padding: 0 var(--space-6); }

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
  .btn-anchor { display: inline-flex; }

  .narrator-block { text-align: center; margin: var(--space-6) 0; }
  .narrator-text {
    margin: var(--space-2) 0 0;
    font-family: var(--font-serif);
    font-size: var(--text-lead);
    line-height: var(--leading-snug);
    color: var(--text-primary);
  }

  .topics {
    display: flex; flex-wrap: wrap; gap: var(--space-2);
    justify-content: center; margin: var(--space-4) 0;
  }

  .page-label {
    margin-top: var(--space-10); text-align: center;
    font-family: var(--font-sans); font-size: var(--text-meta);
    color: var(--text-muted);
  }

  .extra-section { margin-top: var(--space-8); }
  .chips { display: flex; flex-wrap: wrap; gap: var(--space-2); }

  .chain-col { min-width: 0; margin-top: var(--space-8); }

  .parallel-note {
    font-size: var(--text-meta);
    color: var(--text-muted);
    font-style: italic;
    margin: 0 0 var(--space-3) 0;
  }
  .similar-list { display: flex; flex-direction: column; gap: 0; }
  .similar-item {
    display: block; padding: var(--space-4) 0;
    border-bottom: 1px solid var(--border-subtle);
    text-decoration: none; color: inherit;
    transition: background var(--transition);
  }
  .similar-item:hover { background: var(--bg-hover); }
  .similar-item:last-child { border-bottom: none; }
  .similar-meta { display: flex; align-items: center; gap: var(--space-2); margin-bottom: var(--space-2); }
  .similar-ref { font-family: var(--font-mono); font-size: var(--text-meta); color: var(--text-muted); }

  .ayah-list { display: flex; flex-direction: column; gap: var(--space-3); }
  .ayah-item {
    display: block; padding: var(--space-4);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    text-decoration: none; color: inherit;
    transition: border-color var(--transition);
  }
  .ayah-item:hover { border-color: var(--accent); }
  .ayah-meta { margin-bottom: var(--space-2); }
  .ayah-ref { font-family: var(--font-mono); font-size: var(--text-meta); color: var(--accent); }
  .ayah-text { color: var(--text-primary); }
  .ayah-text-en {
    margin-top: var(--space-2); font-family: var(--font-serif);
    color: var(--text-secondary); line-height: var(--leading-relaxed);
  }
</style>
