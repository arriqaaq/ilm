<script lang="ts">
  import { getSurahs, getQuranStats } from '$lib/api';
  import type { ApiSurah, QuranStatsResponse } from '$lib/types';
  import SurahRow from '$lib/components/quran/SurahRow.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import MetaRow from '$lib/components/common/MetaRow.svelte';
  import TabStrip from '$lib/components/layout/TabStrip.svelte';

  let surahs: ApiSurah[] = $state([]);
  let stats: QuranStatsResponse | null = $state(null);
  let loading = $state(true);
  let filter = $state('');
  let view: 'surah' | 'juz' | 'revelation' = $state('surah');
  let sortDir: 'asc' | 'desc' = $state('asc');

  $effect(() => {
    Promise.all([getSurahs(), getQuranStats()]).then(([s, st]) => {
      surahs = s;
      stats = st;
      loading = false;
    });
  });

  let filtered = $derived(() => {
    let list = surahs;
    if (filter.trim()) {
      const q = filter.toLowerCase();
      list = list.filter(s =>
        s.name_translit.toLowerCase().includes(q) ||
        s.name_en.toLowerCase().includes(q) ||
        s.name_ar.includes(filter) ||
        String(s.surah_number) === q
      );
    }
    if (view === 'revelation') {
      list = [...list].sort((a, b) => {
        if (a.revelation_type !== b.revelation_type) {
          return a.revelation_type === 'Meccan' ? -1 : 1;
        }
        return a.surah_number - b.surah_number;
      });
    } else {
      list = [...list].sort((a, b) => a.surah_number - b.surah_number);
    }
    if (sortDir === 'desc') list = list.reverse();
    return list;
  });
</script>

<div class="quran-page">
  <header class="page-header">
    <Eyebrow>Qurʾān</Eyebrow>
    <h1>The Noble Qurʾān</h1>
    {#if stats}
      <MetaRow items={[
        `${stats.surah_count} Surahs`,
        `${stats.ayah_count} Ayahs`
      ]} />
    {/if}
  </header>

  <div class="controls">
    <input
      type="text"
      placeholder="Search surahs…"
      bind:value={filter}
      class="search-input"
    />
  </div>

  <div class="view-row">
    <div class="tab-wrap">
      <TabStrip
        ariaLabel="Browse mode"
        bind:active={view}
        tabs={[
          { id: 'surah', label: 'Surah' },
          { id: 'juz', label: 'Juz', disabled: true },
          { id: 'revelation', label: 'Revelation Order' },
        ]}
      />
    </div>

    <button
      class="sort-btn"
      type="button"
      onclick={() => (sortDir = sortDir === 'asc' ? 'desc' : 'asc')}
      aria-label="Toggle sort direction"
    >
      <span class="sort-label">Sort by:</span>
      <span class="sort-value">{sortDir === 'asc' ? 'Ascending' : 'Descending'}</span>
      <span class="sort-arrow" class:flip={sortDir === 'desc'}>↑</span>
    </button>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else}
    <div class="surah-grid">
      {#each filtered() as surah (surah.id)}
        <SurahRow {surah} />
      {/each}
      {#if filtered().length === 0}
        <div class="empty">No surahs match "{filter}"</div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .quran-page {
    padding: var(--space-8) var(--space-6) var(--space-12);
    max-width: var(--page-width);
    margin: 0 auto;
  }

  .page-header { margin-bottom: var(--space-6); }
  .page-header h1 {
    font-family: var(--font-serif);
    font-size: 2.1rem;
    margin: var(--space-2) 0;
    letter-spacing: var(--tracking-tight);
  }

  .controls {
    display: flex;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
    align-items: center;
    flex-wrap: wrap;
  }
  .search-input {
    flex: 1;
    max-width: 480px;
    min-width: 200px;
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    outline: none;
  }
  .search-input:focus { border-color: var(--accent); }

  .view-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    margin-bottom: var(--space-5);
    border-bottom: 1px solid var(--border-subtle);
    padding-bottom: var(--space-3);
    flex-wrap: wrap;
  }
  .tab-wrap { flex: 1 1 auto; min-width: 0; }

  .sort-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    transition: all var(--transition);
  }
  .sort-btn:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
  }
  .sort-label {
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .sort-value {
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    color: var(--accent);
  }
  .sort-arrow {
    color: var(--accent);
    transition: transform var(--transition);
    font-size: var(--text-meta);
  }
  .sort-arrow.flip { transform: rotate(180deg); }

  .surah-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    gap: var(--space-4);
  }
  .empty {
    grid-column: 1 / -1;
    text-align: center;
    color: var(--text-muted);
    padding: var(--space-10);
    font-family: var(--font-serif);
    font-style: italic;
  }

  @media (max-width: 480px) {
    .surah-grid { grid-template-columns: 1fr; gap: var(--space-3); }
    .view-row { gap: var(--space-3); }
  }
</style>
