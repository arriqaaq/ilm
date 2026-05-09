<script lang="ts">
  type ColorVariant = 'walnut' | 'sienna' | 'malachite' | 'saffron' | 'lapis' | 'aubergine';
  type PatternId = 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10;

  let { title, subtitle, pattern, color, href }: {
    title: string;
    subtitle?: string;
    pattern: PatternId;
    color: ColorVariant;
    href: string;
  } = $props();
</script>

<a class="card color-{color}" {href} style="background-image: url('/patterns/{pattern}.svg');">
  <span class="overlay overlay-base"></span>
  <span class="overlay overlay-blur"></span>
  <h3 class="card-title">{title}</h3>
  {#if subtitle}
    <p class="card-subtitle">{subtitle}</p>
  {/if}
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

  .card-title {
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
  .card-subtitle {
    margin: var(--space-1) 0 0;
    font-size: var(--text-sm);
    color: rgba(255, 255, 255, 0.82);
  }
</style>
