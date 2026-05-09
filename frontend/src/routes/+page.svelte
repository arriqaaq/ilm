<script lang="ts">
  import { onMount } from 'svelte';
  import { getStats, getCollections, getQuranStats, getSurahs, getBooksList } from '$lib/api';
  import type { StatsResponse, ApiCollection, QuranStatsResponse, ApiSurah, Book } from '$lib/types';
  import HeroPanel from '$lib/components/landing/HeroPanel.svelte';
  import HomeSection from '$lib/components/landing/HomeSection.svelte';
  import CollectionCard from '$lib/components/landing/CollectionCard.svelte';
  import BookCoverCard from '$lib/components/landing/BookCoverCard.svelte';
  import SurahPatternCard from '$lib/components/landing/SurahPatternCard.svelte';

  type CollectionColor = 'walnut' | 'sienna' | 'malachite' | 'saffron' | 'lapis' | 'aubergine';
  type PatternId = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10;

  let hadithStats: StatsResponse | null = $state(null);
  let quranStats: QuranStatsResponse | null = $state(null);
  let collections: ApiCollection[] = $state([]);
  let surahs: ApiSurah[] = $state([]);
  let allBooks: Book[] = $state([]);

  const surahVariants: { color: CollectionColor; pattern: PatternId }[] = [
    { color: 'malachite', pattern: 1 },
    { color: 'lapis',     pattern: 2 },
    { color: 'saffron',   pattern: 3 },
    { color: 'sienna',    pattern: 4 },
    { color: 'aubergine', pattern: 5 },
    { color: 'walnut',    pattern: 6 },
  ];

  const hadithVariants: { color: CollectionColor; pattern: PatternId }[] = [
    { color: 'malachite', pattern: 1 },
    { color: 'sienna',    pattern: 2 },
    { color: 'lapis',     pattern: 3 },
    { color: 'saffron',   pattern: 4 },
    { color: 'walnut',    pattern: 5 },
    { color: 'aubergine', pattern: 6 },
  ];

  const bookVariants: { color: CollectionColor; pattern: PatternId }[] = [
    { color: 'lapis',     pattern: 7 },
    { color: 'saffron',   pattern: 8 },
    { color: 'malachite', pattern: 9 },
    { color: 'sienna',    pattern: 10 },
    { color: 'walnut',    pattern: 1 },
    { color: 'aubergine', pattern: 2 },
    { color: 'lapis',     pattern: 3 },
    { color: 'saffron',   pattern: 4 },
    { color: 'malachite', pattern: 5 },
    { color: 'sienna',    pattern: 6 },
    { color: 'walnut',    pattern: 7 },
    { color: 'aubergine', pattern: 8 },
  ];

  const generations: { value: string; nameEn: string; nameAr: string; color: CollectionColor; pattern: PatternId }[] = [
    { value: '1', nameEn: 'Companions',     nameAr: 'الصحابة',        color: 'sienna',    pattern: 7 },
    { value: '2', nameEn: 'Successors',     nameAr: 'التابعون',       color: 'malachite', pattern: 8 },
    { value: '3', nameEn: 'Followers',      nameAr: 'تبع التابعين',   color: 'saffron',   pattern: 9 },
    { value: '4', nameEn: '4th Generation', nameAr: 'الطبقة الرابعة', color: 'lapis',     pattern: 10 },
    { value: '5', nameEn: '5th Generation', nameAr: 'الطبقة الخامسة', color: 'walnut',    pattern: 1 },
    { value: '6', nameEn: '6th Generation', nameAr: 'الطبقة السادسة', color: 'aubergine', pattern: 2 },
  ];

  const featuredSurahNumbers = [1, 2, 18, 36, 55, 67];

  let featuredSurahs = $derived(
    featuredSurahNumbers
      .map(n => surahs.find(s => s.surah_number === n))
      .filter((s): s is ApiSurah => !!s)
  );

  let collectionNameSet = $derived(
    new Set(collections.map(c => c.name_en?.toLowerCase()).filter(Boolean) as string[])
  );

  let otherBooks = $derived(
    allBooks
      .filter(b => !collectionNameSet.has(b.name_en?.toLowerCase() ?? ''))
      .slice(0, 12)
  );

  onMount(async () => {
    try {
      [hadithStats, quranStats, collections, surahs, allBooks] = await Promise.all([
        getStats(), getQuranStats(), getCollections(), getSurahs(), getBooksList(),
      ]);
    } catch (e) { console.error('Failed to load landing data:', e); }
  });
</script>

<svelte:head>
  <title>Ilm — Search the Qurʾān &amp; Sunnah</title>
</svelte:head>

<div class="landing">
  <HeroPanel />

  <main class="sections">
    {#if featuredSurahs.length > 0}
      <HomeSection title="Browse the Qurʾān" href="/quran">
        {#each featuredSurahs as s, i (s.surah_number)}
          {@const v = surahVariants[i % surahVariants.length]}
          <SurahPatternCard
            number={s.surah_number}
            nameAr={s.name_ar}
            nameEn={s.name_en}
            ayahCount={s.ayah_count}
            color={v.color}
            pattern={v.pattern}
            href={`/quran/${s.surah_number}`}
          />
        {/each}
      </HomeSection>
    {/if}

    {#if collections.length > 0}
      <HomeSection title="Hadith Collections" href="/hadiths">
        {#each collections.slice(0, 6) as c, i (c.id)}
          {@const v = hadithVariants[i % hadithVariants.length]}
          <BookCoverCard
            title={c.name_en}
            subtitle={c.name_ar ?? undefined}
            color={v.color}
            pattern={v.pattern}
            href={`/hadiths?book=${c.collection_id}`}
          />
        {/each}
      </HomeSection>
    {/if}

    {#if otherBooks.length > 0}
      <HomeSection title="Library" href="/books">
        {#each otherBooks as b, i (b.book_id)}
          {@const v = bookVariants[i % bookVariants.length]}
          <BookCoverCard
            title={b.name_en || b.name_ar}
            subtitle={b.author_ar || b.name_ar}
            color={v.color}
            pattern={v.pattern}
            href={`/books/${b.book_id}`}
          />
        {/each}
      </HomeSection>
    {/if}

    <HomeSection title="Generations of Narrators" href="/narrators">
      {#each generations as g}
        <CollectionCard
          title={g.nameEn} subtitle={g.nameAr}
          color={g.color} pattern={g.pattern}
          href={`/narrators?generation=${g.value}`}
        />
      {/each}
    </HomeSection>

    {#if hadithStats || quranStats}
      <section class="stats-strip">
        {#if quranStats}
          <div class="stat"><span class="num">{quranStats.surah_count}</span><span class="label">Surahs</span></div>
          <div class="stat"><span class="num">{quranStats.ayah_count.toLocaleString()}</span><span class="label">Ayahs</span></div>
        {/if}
        {#if hadithStats}
          <div class="stat"><span class="num">{hadithStats.hadith_count.toLocaleString()}</span><span class="label">Hadiths</span></div>
          <div class="stat"><span class="num">{hadithStats.narrator_count.toLocaleString()}</span><span class="label">Narrators</span></div>
        {/if}
      </section>
    {/if}
  </main>

  <footer class="landing-footer">
    <p class="footer-line">Built for Islamic scholarship · open data · open source</p>
    <p class="footer-line muted">Hadith data from Sunnah.com · Qurʾān from QUL Tarteel · Tafsir Ibn Kathir · narrator biographies from Tahdhib al-Tahdhib</p>
  </footer>
</div>

<style>
  .landing { min-height: 100vh; background: var(--bg-primary); color: var(--text-primary); }
  .sections { max-width: var(--page-width); margin: 0 auto; padding: var(--space-12) var(--space-6) var(--space-10); }

  .stats-strip {
    display: flex; flex-wrap: wrap; gap: var(--space-8);
    justify-content: center; align-items: baseline;
    padding: var(--space-8) 0;
    border-top: 1px solid var(--border-subtle);
    margin-top: var(--space-6);
  }
  .stat { display: flex; flex-direction: column; align-items: center; gap: var(--space-1); }
  .stat .num {
    font-family: var(--font-serif); font-size: 1.8rem;
    font-weight: var(--font-weight-semibold); color: var(--text-primary);
    letter-spacing: var(--tracking-tight);
  }
  .stat .label {
    font-family: var(--font-sans); font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold); letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase; color: var(--text-muted);
  }

  .landing-footer {
    max-width: var(--page-width); margin: 0 auto;
    padding: var(--space-8) var(--space-6) var(--space-12);
    text-align: center;
    border-top: 1px solid var(--border-subtle);
  }
  .footer-line { margin: 0 0 var(--space-2); font-family: var(--font-serif); font-size: var(--text-body); color: var(--text-secondary); }
  .footer-line.muted { font-style: italic; font-size: var(--text-meta); color: var(--text-muted); }
</style>
