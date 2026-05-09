<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    sidebarWidth = 240,
    sticky = false,
    sidebarSide = 'right',
    fill = false,
    main,
    sidebar,
  }: {
    sidebarWidth?: number;
    sticky?: boolean;
    sidebarSide?: 'left' | 'right';
    fill?: boolean;
    main: Snippet;
    sidebar: Snippet;
  } = $props();
</script>

<div class="two-col" class:left-aside={sidebarSide === 'left'} class:fill>
  {#if sidebarSide === 'left'}
    <aside class="aside" class:sticky style="--aside-w: {sidebarWidth}px">{@render sidebar()}</aside>
    <div class="main">{@render main()}</div>
  {:else}
    <div class="main">{@render main()}</div>
    <aside class="aside" class:sticky style="--aside-w: {sidebarWidth}px">{@render sidebar()}</aside>
  {/if}
</div>

<style>
  .two-col {
    display: flex;
    gap: var(--space-6);
    align-items: flex-start;
    width: 100%;
    min-width: 0;
  }
  .main {
    flex: 1 1 auto;
    min-width: 0;
  }
  .aside {
    flex: 0 0 var(--aside-w);
    width: var(--aside-w);
    min-width: 0;
  }
  .aside.sticky {
    position: sticky;
    top: var(--space-4);
  }
  .two-col.fill {
    height: 100%;
    align-items: stretch;
  }
  .two-col.fill > .main { height: 100%; min-height: 0; }
  .two-col.fill > .aside { height: 100%; }
  @media (max-width: 1024px) {
    .two-col { flex-direction: column; gap: var(--space-5); }
    .left-aside { flex-direction: column; }
    .aside {
      flex: 1 1 auto;
      width: 100%;
      position: static;
    }
  }
</style>
