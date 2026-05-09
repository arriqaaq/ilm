<script lang="ts">
  type ColorVariant = 'walnut' | 'sienna' | 'malachite' | 'saffron' | 'lapis' | 'aubergine';
  type PatternId = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10;

  let { number, nameAr, nameEn, ayahCount, pattern, color, href }: {
    number: number;
    nameAr: string;
    nameEn: string;
    ayahCount: number;
    pattern: PatternId;
    color: ColorVariant;
    href: string;
  } = $props();
</script>

<a class="card color-{color}" {href} style="background-image: url('/patterns/{pattern}.svg');">
  <span class="overlay overlay-base"></span>
  <span class="overlay overlay-blur"></span>
  <span class="number">{number}</span>
  <h3 class="name-ar arabic-prose" dir="rtl">{nameAr}</h3>
  <p class="name-en">{nameEn}</p>
  <p class="ayahs">{ayahCount} ayāt</p>
</a>

<style>
  .card {
    position: relative;
    isolation: isolate;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    width: 176px;
    height: 176px;
    padding: var(--space-4);
    border-radius: var(--radius-2xl);
    overflow: hidden;
    background-repeat: repeat;
    background-size: 88px 88px;
    color: #ffffff;
    text-decoration: none;
    transition: transform var(--transition);
  }
  .card:hover { transform: translateY(-2px); }

  .color-walnut    { background-color: var(--collection-walnut); }
  .color-sienna    { background-color: var(--collection-sienna); }
  .color-malachite { background-color: var(--collection-malachite); }
  .color-saffron   { background-color: var(--collection-saffron); }
  .color-lapis     { background-color: var(--collection-lapis); }
  .color-aubergine { background-color: var(--collection-aubergine); }

  .overlay { position: absolute; inset: 0; pointer-events: none; }
  .overlay-base {
    z-index: -2;
    background: linear-gradient(to top, var(--card-color, var(--collection-walnut)) 0%, transparent 100%);
  }
  .color-walnut    .overlay-base { --card-color: var(--collection-walnut); }
  .color-sienna    .overlay-base { --card-color: var(--collection-sienna); }
  .color-malachite .overlay-base { --card-color: var(--collection-malachite); }
  .color-saffron   .overlay-base { --card-color: var(--collection-saffron); }
  .color-lapis     .overlay-base { --card-color: var(--collection-lapis); }
  .color-aubergine .overlay-base { --card-color: var(--collection-aubergine); }

  .overlay-blur {
    top: auto;
    height: 80px;
    z-index: -1;
    backdrop-filter: blur(2px);
    -webkit-mask-image: linear-gradient(to top, black, transparent);
            mask-image: linear-gradient(to top, black, transparent);
  }

  .number {
    position: absolute;
    top: var(--space-3);
    right: var(--space-3);
    font-family: var(--font-serif);
    font-size: 1.5rem;
    font-weight: var(--font-weight-semibold);
    color: rgba(255, 255, 255, 0.9);
    line-height: 1;
    letter-spacing: var(--tracking-tight);
  }

  .name-ar {
    margin: 0;
    font-size: 1.75rem;
    font-weight: var(--font-weight-semibold);
    color: #ffffff;
    line-height: 1.2;
  }
  .name-en {
    margin: var(--space-1) 0 0;
    font-family: var(--font-serif);
    font-size: var(--text-md);
    color: rgba(255, 255, 255, 0.88);
    line-height: 1.2;
  }
  .ayahs {
    margin: var(--space-1) 0 0;
    font-size: var(--text-xs);
    font-weight: var(--font-weight-medium);
    letter-spacing: var(--tracking-wide);
    text-transform: uppercase;
    color: rgba(255, 255, 255, 0.7);
  }
</style>
