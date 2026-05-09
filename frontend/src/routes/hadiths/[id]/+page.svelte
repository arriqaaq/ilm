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
  import MetaRow from '$lib/components/common/MetaRow.svelte';
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
    loading = true;
    sharhPage = null;
    gradings = [];
    Promise.all([getHadith(id), getChainGraph(id), getHadithGradings(id)])
      .then(([d, g, gr]) => {
        data = d; graphData = g; gradings = gr.gradings;
        // Fetch sharh mapping for this hadith
        if (d.hadith.hadith_number && d.hadith.collection_id) {
          getHadithSharhPages(d.hadith.collection_id, [d.hadith.hadith_number])
            .then(res => {
              const ref = res.mappings[String(d.hadith.hadith_number)];
              if (ref) sharhPage = ref;
            })
            .catch(() => {});
        }
      })
      .catch((e) => console.error('Failed to load hadith:', e))
      .finally(() => { loading = false; });
  });
</script>

<div class="hadith-view prose">
  {#if loading}
    <LoadingSpinner />
  {:else if data}
    <header class="view-header">
      <Eyebrow>
        ḤADĪTH
        {#if data.hadith.book_name}· {data.hadith.book_name}{/if}
        · #{data.hadith.hadith_number}
      </Eyebrow>
      <h1 class="title">Hadith #{data.hadith.hadith_number}</h1>
      <MetaRow items={[data.hadith.chapter_name, data.hadith.hadith_type]} />
      <div class="actions">
        <span class="btn-anchor" bind:this={saveBtnEl}>
          <Button
            variant="ghost"
            size="sm"
            onclick={() => { showSavePopover = !showSavePopover; }}
          >
            ♡ Save
          </Button>
        </span>
        <Button variant="ghost" size="sm" onclick={() => { showNotePanel = true; }}>
          ✎ Note
        </Button>
        {#if sharhPage}
          {@const sp = sharhPage}
          <Button
            variant="secondary"
            size="sm"
            onclick={() => { sharhTarget = { bookId: sp.book_id, pageIndex: sp.page_index, bookName: sp.book_name, hadithNumber: data?.hadith.hadith_number ?? 0 }; }}
          >
            Sharḥ →
          </Button>
        {/if}
      </div>
    </header>

    <Divider variant="hairline" />

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

    <div class="text-section">
      <HadithBody
        textAr={data.hadith.text_ar}
        textEn={data.hadith.text_en}
        language={$language}
        arabicSize={$proseArabicFontSize}
        englishSize={$preferences.englishFontSize}
      />
    </div>

    {#if data.narrators.length > 0}
      <Divider variant="ornamental" />
      <section class="section">
        <SectionHeading eyebrow="Narrators" title="Transmitters" level={2} />
        <div class="chips">
          {#each data.narrators as narrator}
            <NarratorChip {narrator} />
          {/each}
        </div>
      </section>
    {/if}

    <Divider variant="ornamental" />
    <div class="chain-with-panel">
      <section class="section chain-col">
        <SectionHeading eyebrow="Isnād" title="Narrator Chain" level={2} />
        <ChainView data={graphData} />
      </section>
      <aside class="grading-col">
        <GradingPanel {gradings} />
      </aside>
    </div>

    {#if data.linked_ayahs && data.linked_ayahs.length > 0}
      <Divider variant="ornamental" />
      <section class="section">
        <SectionHeading eyebrow="References" title="Quranic Verses" level={2} />
        <div class="ayah-list">
          {#each data.linked_ayahs as ayah}
            <a href="/quran/{ayah.surah_number}" class="ayah-item">
              <div class="ayah-meta">
                <span class="ayah-ref">{ayah.surah_number}:{ayah.ayah_number}</span>
              </div>
              <div class="ayah-text arabic-prose" dir="rtl" style="font-size: {$preferences.arabicFontSize}rem">{ayah.text_ar}</div>
              {#if ayah.text_en}
                <div class="ayah-text-en" style="font-size: {$preferences.englishFontSize}rem">{ayah.text_en}</div>
              {/if}
            </a>
          {/each}
        </div>
      </section>
    {/if}

    {#if data.similar_hadiths && data.similar_hadiths.length > 0}
      <Divider variant="ornamental" />
      <section class="section">
        <SectionHeading eyebrow="Similar" title="Related Hadiths" level={2} />
        <div class="similar-list">
          {#each data.similar_hadiths as similar}
            <a href="/hadiths/{similar.id}" class="similar-item">
              <div class="similar-meta">
                <span class="similar-ref">#{similar.hadith_number}</span>
                {#if similar.book_name}
                  <Badge text={similar.book_name} variant="accent" />
                {/if}
              </div>
              <HadithBody
                textAr={similar.text_ar}
                textEn={similar.text_en}
                language={$language}
                arabicSize={$proseArabicFontSize}
                englishSize={1}
                preview
                previewLength={150}
              />
            </a>
          {/each}
        </div>
      </section>
    {/if}
  {:else}
    <div class="empty">Hadith not found.</div>
  {/if}
</div>

{#if showSavePopover && data}
  <SavePopover
    refType="hadith"
    refId={data.hadith.id}
    refLabel="Hadith #{data.hadith.hadith_number}"
    anchorX={saveBtnEl ? saveBtnEl.getBoundingClientRect().left : 100}
    anchorY={saveBtnEl ? saveBtnEl.getBoundingClientRect().bottom + 4 : 100}
    onclose={() => { showSavePopover = false; }}
  />
{/if}

{#if showNotePanel && data}
  <NoteModal
    refType="hadith"
    refId={data.hadith.id}
    refLabel="Hadith #{data.hadith.hadith_number}"
    onclose={() => { showNotePanel = false; }}
  />
{/if}

{#if sharhTarget}
  <BookViewerModal
    bookId={sharhTarget.bookId}
    pageIndex={sharhTarget.pageIndex}
    title={sharhTarget.bookName}
    subtitle="Hadith {sharhTarget.hadithNumber}"
    onclose={() => { sharhTarget = null; }}
  />
{/if}

<style>
  .hadith-view {
    padding: var(--space-8) var(--space-6);
    margin: 0 auto;
  }
  .hadith-view.prose { max-width: var(--prose-width); }

  .view-header { margin-bottom: var(--space-6); }
  .view-header .title {
    font-family: var(--font-serif);
    font-size: 2.1rem;
    line-height: var(--leading-tight);
    letter-spacing: var(--tracking-tight);
    margin: var(--space-2) 0 var(--space-3);
  }
  .actions {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-4);
    flex-wrap: wrap;
    align-items: center;
  }
  .btn-anchor { display: inline-flex; }

  .narrator-block { margin: var(--space-6) 0; }
  .narrator-text {
    margin: var(--space-2) 0 0;
    font-family: var(--font-serif);
    font-size: var(--text-lead);
    line-height: var(--leading-snug);
    color: var(--text-primary);
  }

  .topics {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin: var(--space-4) 0;
  }

  .text-section {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    margin: var(--space-6) 0;
  }

  .section { margin-bottom: var(--space-6); }
  .chips { display: flex; flex-wrap: wrap; gap: var(--space-2); }

  .chain-with-panel {
    display: flex;
    gap: var(--space-6);
    align-items: flex-start;
    margin-bottom: var(--space-6);
  }
  .chain-col { flex: 1 1 auto; min-width: 0; margin-bottom: 0; }
  .grading-col {
    flex: 0 0 340px;
    position: sticky;
    top: var(--space-4);
  }
  @media (max-width: 900px) {
    .chain-with-panel { flex-direction: column; }
    .grading-col { flex: 1 1 auto; position: static; }
  }

  .similar-list { display: flex; flex-direction: column; gap: 0; }
  .similar-item {
    display: block;
    padding: var(--space-4) 0;
    border-bottom: 1px solid var(--border-subtle);
    text-decoration: none;
    color: inherit;
    transition: background var(--transition);
  }
  .similar-item:hover { background: var(--bg-hover); }
  .similar-item:last-child { border-bottom: none; }
  .similar-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }
  .similar-ref {
    font-family: var(--font-mono);
    font-size: var(--text-meta);
    color: var(--text-muted);
  }

  .ayah-list { display: flex; flex-direction: column; gap: var(--space-3); }
  .ayah-item {
    display: block;
    padding: var(--space-4);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    text-decoration: none;
    color: inherit;
    transition: border-color var(--transition);
  }
  .ayah-item:hover { border-color: var(--accent); }
  .ayah-meta { margin-bottom: var(--space-2); }
  .ayah-ref {
    font-family: var(--font-mono);
    font-size: var(--text-meta);
    color: var(--accent);
  }
  .ayah-text {
    color: var(--text-primary);
  }
  .ayah-text-en {
    margin-top: var(--space-2);
    font-family: var(--font-serif);
    color: var(--text-secondary);
    line-height: var(--leading-relaxed);
  }

  .empty { text-align: center; color: var(--text-muted); padding: var(--space-10); }
</style>
