<script lang="ts">
  import { getSurahs, getQuranStats } from '$lib/api';
  import type { ApiSurah, QuranStatsResponse } from '$lib/types';
  import SurahRow from '$lib/components/quran/SurahRow.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import MetaRow from '$lib/components/common/MetaRow.svelte';

  let surahs: ApiSurah[] = $state([]);
  let stats: QuranStatsResponse | null = $state(null);
  let loading = $state(true);
  let filter = $state('');
  let sortBy: 'number' | 'revelation' = $state('number');

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
    if (sortBy === 'revelation') {
      // Meccan first, then Medinan
      list = [...list].sort((a, b) => {
        if (a.revelation_type !== b.revelation_type) {
          return a.revelation_type === 'Meccan' ? -1 : 1;
        }
        return a.surah_number - b.surah_number;
      });
    }
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
    <input type="text" placeholder="Search surahs…" bind:value={filter} class="search-input" />
    <div class="sort-toggle">
      <button class="toggle-btn" class:active={sortBy === 'number'} onclick={() => sortBy = 'number'}>Surah</button>
      <button class="toggle-btn" class:active={sortBy === 'revelation'} onclick={() => sortBy = 'revelation'}>Revelation</button>
    </div>
  </div>

  {#if loading}
    <LoadingSpinner />
  {:else}
    <div class="surah-list">
      {#each filtered() as surah}
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
    padding: var(--space-8) var(--space-6);
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
    margin-bottom: var(--space-5);
    align-items: center;
    flex-wrap: wrap;
  }
  .search-input { flex: 1; max-width: 480px; min-width: 200px; }
  .sort-toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .toggle-btn {
    padding: var(--space-2) var(--space-4);
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    font-weight: var(--font-weight-medium);
    background: transparent;
    color: var(--text-secondary);
    transition: all var(--transition);
  }
  .toggle-btn.active {
    background: var(--accent-muted);
    color: var(--accent);
  }
  .surah-list {
    border-top: 1px solid var(--border-subtle);
  }
  .empty { text-align: center; color: var(--text-muted); padding: var(--space-10); }
</style>
