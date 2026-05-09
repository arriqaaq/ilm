<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    children,
    lines = 3,
  }: {
    children: Snippet;
    lines?: number;
  } = $props();

  let el: HTMLDivElement | undefined = $state();
  let expanded = $state(false);
  let canToggle = $state(false);

  $effect(() => {
    if (!el) return;
    // Recompute when the rendered content changes; clamped state always wins.
    if (!expanded) {
      canToggle = el.scrollHeight > el.clientHeight + 1;
    }
  });
</script>

<div class="wrap">
  <div
    bind:this={el}
    class="text"
    class:clamped={!expanded}
    style:--lines={lines}
  >
    {@render children()}
  </div>
  {#if canToggle}
    <button
      type="button"
      class="toggle"
      onclick={() => (expanded = !expanded)}
    >
      {expanded ? 'See less' : 'See more'}
    </button>
  {/if}
</div>

<style>
  .wrap { width: 100%; }
  .text {
    font-family: var(--font-serif);
    color: var(--text-secondary);
    line-height: var(--leading-normal);
    font-size: var(--text-body);
  }
  .text.clamped {
    display: -webkit-box;
    -webkit-line-clamp: var(--lines);
    line-clamp: var(--lines);
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
  .toggle {
    margin-top: var(--space-2);
    background: transparent;
    border: none;
    padding: 0;
    color: var(--accent);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    cursor: pointer;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .toggle:hover { color: var(--accent-hover); }
</style>
