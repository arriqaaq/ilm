<script lang="ts">
  import type { Snippet } from 'svelte';

  let { title, href, children }: {
    title: string;
    href?: string;
    children: Snippet;
  } = $props();

  let scroller: HTMLDivElement | undefined = $state();

  function scrollByAmount(delta: number) {
    if (scroller) scroller.scrollBy({ left: delta, behavior: 'smooth' });
  }
</script>

<section class="home-section">
  <div class="section-header">
    {#if href}
      <a class="title-link" {href}>
        <h2 class="title">{title}</h2>
        <span class="chevron" aria-hidden="true">→</span>
      </a>
    {:else}
      <h2 class="title">{title}</h2>
    {/if}
    <div class="arrows">
      <button class="arrow" type="button" aria-label="Scroll left" onclick={() => scrollByAmount(-220)}>‹</button>
      <button class="arrow" type="button" aria-label="Scroll right" onclick={() => scrollByAmount(220)}>›</button>
    </div>
  </div>

  <div class="scroller" bind:this={scroller}>
    {@render children()}
  </div>
</section>

<style>
  .home-section { padding-bottom: var(--space-10); }
  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-5);
  }
  .title-link {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    text-decoration: none;
    color: inherit;
    transition: color var(--transition);
  }
  .title-link:hover { color: var(--accent); }
  .title-link:hover .chevron { transform: translateX(2px); }
  .title {
    font-family: var(--font-serif);
    font-size: 1.6rem;
    font-weight: var(--font-weight-semibold);
    margin: 0;
    letter-spacing: var(--tracking-tight);
  }
  .chevron {
    font-size: 1.2rem;
    color: var(--text-muted);
    transition: transform var(--transition);
  }
  .arrows { display: flex; gap: var(--space-2); }
  .arrow {
    width: 32px; height: 32px;
    display: inline-flex; align-items: center; justify-content: center;
    border-radius: var(--radius-full);
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-size: 1rem; line-height: 1;
    cursor: pointer;
    transition: all var(--transition);
  }
  .arrow:hover { background: var(--accent-muted); color: var(--accent); border-color: var(--accent); }

  .scroller {
    display: flex;
    gap: var(--space-4);
    overflow-x: auto;
    scroll-snap-type: x mandatory;
    scroll-padding: var(--space-1);
    padding-bottom: var(--space-2);
    -ms-overflow-style: none;
    scrollbar-width: none;
  }
  .scroller::-webkit-scrollbar { display: none; }
  .scroller > :global(*) { scroll-snap-align: start; flex-shrink: 0; }
</style>
