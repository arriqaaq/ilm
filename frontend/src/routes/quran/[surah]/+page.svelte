<script lang="ts">
  import { page } from '$app/state';
  import { getSurah, fetchNoteRefs, getSurahTafsirPages } from '$lib/api';
  import type { ApiAyah, ApiAyahSearchResult, SurahDetailResponse, NoteRefsIndicator, TafsirPageRef } from '$lib/types';
  import AyahCard from '$lib/components/quran/AyahCard.svelte';
  import PageHero from '$lib/components/layout/PageHero.svelte';
  import AyahDetailsModal from '$lib/components/quran/AyahDetailsModal.svelte';
  import NoteModal from '$lib/components/notes/NoteModal.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import RecitationPlayer from '$lib/components/quran/RecitationPlayer.svelte';
  import BookViewerModal from '$lib/components/reader/BookViewerModal.svelte';
  import SurahSidebar from '$lib/components/quran/SurahSidebar.svelte';
  import SidebarTabs from '$lib/components/reader/SidebarTabs.svelte';
  import BookChat from '$lib/components/reader/BookChat.svelte';
  import ResizeHandle from '$lib/components/layout/ResizeHandle.svelte';
  import { loadBooksConfig, getBookConfig, getTafsirBookId } from '$lib/stores/books';
  import { appConfig } from '$lib/stores/config';
  import type { BooksConfig } from '$lib/types';
  import { preferences } from '$lib/stores/preferences';

  let booksConfig: BooksConfig | null = $state(null);
  let tafsirBookId: number | null = $derived(booksConfig ? getTafsirBookId(booksConfig) : null);
  let tafsirBookConfig = $derived(
    tafsirBookId && booksConfig ? getBookConfig(booksConfig, tafsirBookId) : undefined
  );
  let tafsirBookName = $derived(tafsirBookConfig?.name_en ?? 'Tafsir');
  let tafsirDefaultQuestions = $derived(tafsirBookConfig?.default_questions ?? []);

  let data: SurahDetailResponse | null = $state(null);
  let loading = $state(true);
  let activeAyah = $state(0);
  let panelAyah: ApiAyah | ApiAyahSearchResult | null = $state(null);
  let noteTarget: { refType: 'ayah'; refId: string; label: string } | null = $state(null);
  let noteIndicators: NoteRefsIndicator = $state({});
  let tafsirMappings: Record<string, TafsirPageRef> = $state({});
  let tafsirTarget: { pageIndex: number; ayahRef: string } | null = $state(null);
  let playerRef: ReturnType<typeof RecitationPlayer> | undefined = $state(undefined);

  // Right sidebar state
  let rightCollapsed = $state(true);
  let rightWidth = $state(360);
  let rightDragging = $state(false);
  const RIGHT_MIN = 220;
  const RIGHT_MAX = 480;
  const RIGHT_COLLAPSED_W = 40;

  let surahNum = $derived(Number(page.params.surah));
  let startingAyah = $derived(Number(page.url.searchParams.get('ayah')) || 0);
  let reciterFolder = $derived($preferences.selectedReciter ?? 'Alafasy_128kbps');

  function saveRightState() {
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('quran_right_sidebar', JSON.stringify({ collapsed: rightCollapsed, width: rightWidth }));
    }
  }

  $effect(() => {
    if (typeof localStorage !== 'undefined') {
      try {
        const saved = JSON.parse(localStorage.getItem('quran_right_sidebar') ?? '{}');
        if (typeof saved.collapsed === 'boolean') rightCollapsed = saved.collapsed;
        if (typeof saved.width === 'number') rightWidth = Math.max(RIGHT_MIN, Math.min(saved.width, RIGHT_MAX));
      } catch { /* ignore */ }
    }
  });

  $effect(() => {
    loadBooksConfig().then((c) => { booksConfig = c; });
  });

  $effect(() => {
    loading = true;
    activeAyah = 0;
    getSurah(surahNum).then((d) => {
      data = d;
      loading = false;

      const refIds = d.ayahs.map((a: ApiAyah) => `${d.surah.surah_number}:${a.ayah_number}`);
      fetchNoteRefs('ayah', refIds)
        .then(indicators => { noteIndicators = indicators; })
        .catch(() => {});

      getSurahTafsirPages(surahNum)
        .then(res => { tafsirMappings = res.mappings; })
        .catch(() => {});

      if (startingAyah > 0) {
        activeAyah = startingAyah;
        requestAnimationFrame(() => {
          setTimeout(() => {
            const el = document.getElementById(`${surahNum}:${startingAyah}`);
            if (el) {
              el.scrollIntoView({ behavior: 'smooth', block: 'center' });
            }
          }, 100);
        });
      }
    });
  });

  function handleAyahChange(ayah: number) {
    activeAyah = ayah;
    const el = document.getElementById(`${surahNum}:${ayah}`);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'center' });
    }
  }

  function handleAyahPlay(ayah: number) {
    activeAyah = ayah;
    if (playerRef) {
      playerRef.playAyah(ayah);
    }
  }

  function handleRightDrag(deltaX: number) {
    rightWidth = Math.max(RIGHT_MIN, Math.min(rightWidth - deltaX, RIGHT_MAX));
    saveRightState();
  }

  function toggleRight() {
    rightCollapsed = !rightCollapsed;
    saveRightState();
  }

  let mobileDrawerOpen = $state(false);
</script>

<div class="reader">
  {#if loading}
    <div class="reader-loading"><LoadingSpinner /></div>
  {:else if data}
    {@const d = data}
    <div class="reader-body">
      <div class="reader-main">
        <article class="reader-column">
          <PageHero
            nameEn={d.surah.name_translit}
            nameAr={d.surah.name_ar}
            arabicFont="quran"
          >
            {#snippet accent()}<span class="meaning">{d.surah.name_en}</span>{/snippet}
            {#snippet meta()}
              <span class="hero-meta-item"><span class="hero-meta-label">Sūrah</span> <span class="mono">#{d.surah.surah_number}</span></span>
              <span class="hero-dot">·</span>
              <span class="hero-meta-item"><span class="hero-meta-label">Āyāt</span> {d.surah.ayah_count}</span>
              <span class="hero-dot">·</span>
              <span class="hero-meta-item"><span class="hero-meta-label">Revelation</span> {d.surah.revelation_type}</span>
            {/snippet}
          </PageHero>
        </article>

        <div class="ayah-stream">
          {#each d.ayahs as ayah}
            <div id="{d.surah.surah_number}:{ayah.ayah_number}">
              <AyahCard
                {ayah}
                active={ayah.ayah_number === activeAyah}
                onplay={handleAyahPlay}
                onopenpanel={(a) => panelAyah = a}
                onopennote={(a) => { noteTarget = { refType: 'ayah', refId: `${a.surah_number}:${a.ayah_number}`, label: `${a.surah_number}:${a.ayah_number}` }; }}
                onopentafsir={(info) => { tafsirTarget = info; }}
                noteIndicator={noteIndicators[`${d.surah.surah_number}:${ayah.ayah_number}`]}
                tafsirPage={tafsirMappings[String(ayah.ayah_number)]}
                {reciterFolder}
              />
            </div>
          {/each}
        </div>

        <nav class="surah-nav">
          {#if d.surah.surah_number > 1}
            <a href="/quran/{d.surah.surah_number - 1}" class="nav-link">← Previous Sūrah</a>
          {:else}
            <span></span>
          {/if}
          <a href="/quran" class="nav-link">All Sūrahs</a>
          {#if d.surah.surah_number < 114}
            <a href="/quran/{d.surah.surah_number + 1}" class="nav-link">Next Sūrah →</a>
          {:else}
            <span></span>
          {/if}
        </nav>
      </div>

    <!-- Desktop: right sidebar (full page height, animates to 0 when collapsed) -->
    <div class="right-area">
      {#if !rightCollapsed}
        <ResizeHandle
          ondrag={handleRightDrag}
          ondragstart={() => (rightDragging = true)}
          ondragend={() => (rightDragging = false)}
        />
      {/if}
      <div
        class="right-sidebar"
        class:dragging={rightDragging}
        style="width: {rightCollapsed ? 0 : rightWidth}px"
      >
        {#if !rightCollapsed}
          <div class="sidebar-inner" style="width: {rightWidth}px">
            <div class="sidebar-header-bar">
              <button class="sidebar-close-btn" onclick={toggleRight} title="Close panel">
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
              </button>
            </div>
            <div class="sidebar-scroll">
              <SidebarTabs>
                {#snippet content()}
                  <SurahSidebar
                    surah={d.surah}
                    totalAyahs={d.surah.ayah_count}
                    onNavigateAyah={handleAyahChange}
                    onClose={toggleRight}
                  />
                {/snippet}
                {#snippet chat()}
                  {#if $appConfig.advanced_enabled}
                    <BookChat
                      bookId={tafsirBookId ?? 0}
                      bookName={tafsirBookName}
                      currentPageIndex={0}
                      onNavigate={() => {}}
                      defaultQuestions={tafsirDefaultQuestions}
                    />
                  {:else}
                    <p class="chat-disabled-msg">Chat is not available in this build.</p>
                  {/if}
                {/snippet}
              </SidebarTabs>
            </div>
          </div>
        {/if}
      </div>
      <button class="sidebar-open-btn" class:visible={rightCollapsed} onclick={toggleRight} title="Open panel">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="15 18 9 12 15 6"/></svg>
      </button>
    </div>
    </div>

    <!-- Mobile: floating button + drawer -->
    <button class="mobile-sidebar-btn" onclick={() => { mobileDrawerOpen = true; }} aria-label="Open panel">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="15" y2="12"/><line x1="3" y1="18" x2="18" y2="18"/></svg>
    </button>

    {#if mobileDrawerOpen}
      <button class="mobile-backdrop" onclick={() => { mobileDrawerOpen = false; }} aria-label="Close panel"></button>
      <div class="mobile-drawer">
        <SidebarTabs>
          {#snippet content()}
            <SurahSidebar
              surah={d.surah}
              totalAyahs={d.surah.ayah_count}
              onNavigateAyah={(ayah) => { mobileDrawerOpen = false; handleAyahChange(ayah); }}
              onClose={() => { mobileDrawerOpen = false; }}
            />
          {/snippet}
          {#snippet chat()}
            {#if $appConfig.advanced_enabled}
              <BookChat
                bookId={tafsirBookId ?? 0}
                bookName={tafsirBookName}
                currentPageIndex={0}
                onNavigate={() => { mobileDrawerOpen = false; }}
                defaultQuestions={tafsirDefaultQuestions}
              />
            {:else}
              <p style="padding: 16px; color: var(--text-muted); font-size: 0.85rem;">Chat is not available in this build.</p>
            {/if}
          {/snippet}
        </SidebarTabs>
      </div>
    {/if}
  {/if}
</div>

{#if panelAyah}
  <AyahDetailsModal ayah={panelAyah} onclose={() => panelAyah = null} />
{/if}

{#if noteTarget}
  <NoteModal
    refType={noteTarget.refType}
    refId={noteTarget.refId}
    refLabel="Quran {noteTarget.refId}"
    onclose={() => noteTarget = null}
  />
{/if}

{#if tafsirTarget}
  {@const parts = tafsirTarget.ayahRef.split(':').map(Number)}
  <BookViewerModal
    bookId={tafsirBookId ?? 23604}
    pageIndex={tafsirTarget.pageIndex}
    title={tafsirBookName}
    subtitle={tafsirTarget.ayahRef}
    onclose={() => { tafsirTarget = null; }}
    tafsirAnchor={booksConfig && parts.length === 2 ? {
      surah: parts[0],
      ayah: parts[1],
      availableBooks: booksConfig.tafsir_books ?? []
    } : undefined}
  />
{/if}

{#if data}
  {@const d = data}
  <RecitationPlayer
    bind:this={playerRef}
    surahNumber={d.surah.surah_number}
    ayahCount={d.surah.ayah_count}
    onayahchange={handleAyahChange}
  />
{/if}

<style>
  .reader {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--reader-bg);
    color: var(--reader-fg);
    overflow: hidden;
  }
  .reader-loading {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
  }
  .reader-body {
    display: flex;
    flex: 1;
    overflow: hidden;
    min-height: 0;
  }
  .reader-main {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: var(--space-8) 0 var(--space-12);
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }
  .reader-main::-webkit-scrollbar { width: 4px; }
  .reader-main::-webkit-scrollbar-track { background: transparent; }
  .reader-main::-webkit-scrollbar-thumb { background: var(--border); border-radius: 2px; }

  .reader-column {
    max-width: 64rem;
    margin: 0 auto;
    padding: 0 2rem;
  }
  .ayah-stream {
    max-width: 64rem;
    margin: 0 auto;
    padding: 0 2rem;
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }

  @media (min-width: 1280px) {
    .reader-column, .ayah-stream { padding: 0 4rem; }
  }

  .meaning {
    font-family: var(--font-serif);
    font-size: var(--text-meta);
    font-style: italic;
    color: var(--text-muted);
  }
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

  .surah-nav {
    max-width: 64rem;
    margin: var(--space-10) auto 0;
    padding: var(--space-6) 2rem 0;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: var(--space-3);
    border-top: 1px solid var(--border-subtle);
  }
  .nav-link {
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-secondary);
    text-decoration: none;
    transition: color var(--transition);
  }
  .nav-link:hover { color: var(--accent); }

  .right-area {
    display: flex;
    flex-shrink: 0;
    height: 100%;
  }
  .right-sidebar {
    height: 100%;
    overflow: hidden;
    transition: width 200ms ease;
    flex-shrink: 0;
    will-change: width;
  }
  .right-sidebar.dragging {
    transition: none;
  }
  .sidebar-inner {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--bg-surface);
    border-left: 1px solid var(--border-subtle);
  }
  .sidebar-header-bar {
    display: flex;
    justify-content: flex-end;
    padding: var(--space-2);
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }
  .sidebar-close-btn {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    transition: all var(--transition);
  }
  .sidebar-close-btn:hover {
    color: var(--accent);
    background: var(--accent-muted);
  }
  .sidebar-scroll {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .sidebar-open-btn {
    width: 0;
    height: 100%;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-left: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    color: var(--text-muted);
    cursor: pointer;
    transition: width 200ms ease, opacity 200ms ease;
    overflow: hidden;
    opacity: 0;
    padding: 0;
  }
  .sidebar-open-btn.visible {
    width: 32px;
    opacity: 1;
  }
  .sidebar-open-btn:hover {
    color: var(--accent);
    background: var(--accent-muted);
  }

  .chat-disabled-msg {
    padding: var(--space-4);
    color: var(--text-muted);
    font-size: var(--text-meta);
    font-family: var(--font-sans);
    font-style: italic;
  }

  .mobile-sidebar-btn { display: none; }
  .mobile-backdrop { display: none; }
  .mobile-drawer { display: none; }

  @media (max-width: 640px) {
    .reader-column, .ayah-stream { padding: 0 1rem; }
    .surah-nav { padding: var(--space-6) 1rem 0; }
  }

  @media (max-width: 768px) {
    .reader-body { flex-direction: column; }
    .reader-main { padding: var(--space-4) 0 var(--space-12); }
    .right-area { display: none; }

    .mobile-sidebar-btn {
      display: flex;
      align-items: center;
      justify-content: center;
      position: fixed;
      bottom: 80px;
      right: 16px;
      width: 44px;
      height: 44px;
      border-radius: var(--radius-full);
      border: 1px solid var(--border);
      background: var(--bg-surface);
      color: var(--text-secondary);
      box-shadow: 0 2px 8px rgba(0,0,0,0.12);
      cursor: pointer;
      z-index: 30;
      transition: all var(--transition);
    }
    .mobile-sidebar-btn:hover {
      border-color: var(--accent);
      color: var(--accent);
    }

    .mobile-backdrop {
      display: block;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.4);
      z-index: 49;
      border: none;
      padding: 0;
      cursor: default;
    }

    .mobile-drawer {
      display: block;
      position: fixed;
      top: 0;
      right: 0;
      width: 85%;
      max-width: 340px;
      height: 100vh;
      z-index: 50;
      background: var(--bg-primary);
      box-shadow: -4px 0 20px rgba(0, 0, 0, 0.15);
      overflow-y: auto;
    }
  }
</style>
