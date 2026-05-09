<script lang="ts">
  type ColorVariant = 'walnut' | 'sienna' | 'malachite' | 'saffron' | 'lapis' | 'aubergine';
  type PatternId = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10;

  let { title, subtitle, coverUrl, pattern, color, href }: {
    title: string;
    subtitle?: string;
    coverUrl?: string;
    pattern: PatternId;
    color: ColorVariant;
    href: string;
  } = $props();
</script>

<a class="cover color-{color}" {href}
   style={coverUrl ? '' : `background-image: url('/patterns/${pattern}.svg');`}>
  {#if coverUrl}
    <img src={coverUrl} alt={title} loading="lazy" />
  {/if}
  <span class="overlay overlay-base"></span>
  <span class="overlay overlay-blur"></span>
  <div class="text">
    <h3 class="title" dir="auto">{title}</h3>
    {#if subtitle}
      <p class="subtitle" dir="auto">{subtitle}</p>
    {/if}
  </div>
</a>

<style>
  .cover {
    position: relative;
    isolation: isolate;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: center;
    width: 140px;
    aspect-ratio: 1600 / 2300;
    padding: var(--space-4) var(--space-3);
    border-radius: var(--radius-lg);
    overflow: hidden;
    background-repeat: repeat;
    background-size: 88px 88px;
    color: #ffffff;
    text-decoration: none;
    box-shadow: var(--shadow-card);
    transition: transform var(--transition);
  }
  .cover:hover { transform: translateY(-2px); }

  @media (min-width: 640px) { .cover { width: 160px; } }
  @media (min-width: 1024px) { .cover { width: 180px; } }

  .cover img {
    position: absolute; inset: 0;
    width: 100%; height: 100%;
    object-fit: cover;
    display: block;
    z-index: -3;
  }

  .color-walnut    { background-color: var(--collection-walnut); }
  .color-sienna    { background-color: var(--collection-sienna); }
  .color-malachite { background-color: var(--collection-malachite); }
  .color-saffron   { background-color: var(--collection-saffron); }
  .color-lapis     { background-color: var(--collection-lapis); }
  .color-aubergine { background-color: var(--collection-aubergine); }

  .overlay { position: absolute; inset: 0; pointer-events: none; }
  .overlay-base {
    z-index: -2;
    background: linear-gradient(to top, var(--card-color, var(--collection-walnut)) 0%, transparent 60%);
  }
  .color-walnut    .overlay-base { --card-color: var(--collection-walnut); }
  .color-sienna    .overlay-base { --card-color: var(--collection-sienna); }
  .color-malachite .overlay-base { --card-color: var(--collection-malachite); }
  .color-saffron   .overlay-base { --card-color: var(--collection-saffron); }
  .color-lapis     .overlay-base { --card-color: var(--collection-lapis); }
  .color-aubergine .overlay-base { --card-color: var(--collection-aubergine); }

  .overlay-blur {
    top: auto;
    height: 96px;
    z-index: -1;
    backdrop-filter: blur(2px);
    -webkit-mask-image: linear-gradient(to top, black, transparent);
            mask-image: linear-gradient(to top, black, transparent);
  }

  .text {
    position: relative;
    text-align: center;
    width: 100%;
  }
  .title {
    margin: 0;
    font-family: var(--font-serif);
    font-size: 1.35rem;
    font-weight: var(--font-weight-semibold);
    line-height: 1.2;
    color: #ffffff;
    display: -webkit-box;
    -webkit-line-clamp: 2;
            line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .subtitle {
    margin: var(--space-1) 0 0;
    font-size: var(--text-sm);
    color: rgba(255, 255, 255, 0.82);
    display: -webkit-box;
    -webkit-line-clamp: 1;
            line-clamp: 1;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
