<script lang="ts">
  import type { ApiSurah } from '$lib/types';

  let { surah }: { surah: ApiSurah } = $props();
</script>

<a href="/quran/{surah.surah_number}" class="surah-card">
  <div class="number-wrap">
    <span class="number-diamond">
      <span class="number-text">{surah.surah_number}</span>
    </span>
  </div>

  <div class="info">
    <h3 class="name-translit">{surah.name_translit}</h3>
    <p class="name-meaning">{surah.name_en}</p>
  </div>

  <div class="meta">
    <span class="name-ar arabic-prose" dir="rtl">{surah.name_ar}</span>
    <span class="ayah-count">{surah.ayah_count} Ayahs</span>
  </div>
</a>

<style>
  .surah-card {
    display: grid;
    grid-template-columns: 56px 1fr auto;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-5);
    background: var(--bg-secondary);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    color: var(--text-primary);
    text-decoration: none;
    transition: all var(--transition);
    min-height: 88px;
  }
  .surah-card:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .number-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
  }
  /* Rotated-diamond shape behind the number */
  .number-diamond {
    position: relative;
    width: 40px;
    height: 40px;
    background: var(--accent-muted);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    transform: rotate(45deg);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .number-text {
    transform: rotate(-45deg);
    font-family: var(--font-mono);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-semibold);
    color: var(--accent);
  }

  .info {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .name-translit {
    margin: 0;
    font-family: var(--font-serif);
    font-size: var(--text-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    letter-spacing: var(--tracking-tight);
    line-height: 1.2;
  }
  .name-meaning {
    margin: 0;
    font-family: var(--font-serif);
    font-size: var(--text-meta);
    color: var(--text-muted);
    font-style: italic;
  }

  .meta {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    text-align: right;
  }
  .name-ar {
    font-size: 1.4rem;
    color: var(--text-primary);
    line-height: 1.2;
    font-weight: var(--font-weight-semibold);
  }
  .ayah-count {
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-muted);
  }

  @media (max-width: 480px) {
    .surah-card {
      grid-template-columns: 44px 1fr auto;
      padding: var(--space-3) var(--space-4);
      gap: var(--space-3);
    }
    .number-diamond { width: 32px; height: 32px; }
    .name-translit { font-size: var(--text-base); }
    .name-ar { font-size: 1.2rem; }
  }
</style>
