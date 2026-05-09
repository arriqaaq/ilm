<script lang="ts">
  let { number, nameAr, nameEn, ayahCount }: {
    number: number; nameAr: string; nameEn: string; ayahCount?: number;
  } = $props();
</script>

<a class="surah-pill" href="/quran/{number}">
  <span class="num mono">{number}</span>
  <span class="names">
    <span class="name-ar arabic-prose" dir="rtl">{nameAr}</span>
    <span class="name-en">{nameEn}</span>
    {#if ayahCount}<span class="ayah-count">{ayahCount} āyāt</span>{/if}
  </span>
</a>

<style>
  .surah-pill {
    display: inline-flex; align-items: center; gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    background: var(--bg-surface);
    color: var(--text-primary); text-decoration: none;
    transition: all var(--transition);
    min-width: 220px;
  }
  .surah-pill:hover { border-color: var(--accent); background: var(--bg-hover); }
  .num { font-size: var(--text-meta); color: var(--text-muted); min-width: 1.5em; text-align: right; }
  .names { display: flex; flex-direction: column; gap: 2px; }
  .name-ar { font-size: 1.15rem; font-weight: var(--font-weight-semibold); line-height: 1.3; }
  .name-en { font-family: var(--font-serif); font-size: var(--text-meta); color: var(--text-secondary); }
  .ayah-count { font-size: var(--text-2xs); color: var(--text-muted); }
</style>
