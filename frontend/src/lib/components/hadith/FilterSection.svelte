<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    title,
    onClear,
    activeCount = 0,
    children,
  }: {
    title: string;
    onClear?: () => void;
    activeCount?: number;
    children: Snippet;
  } = $props();
</script>

<section class="filter-section">
  <header class="head">
    <h3 class="title">
      {title}{#if activeCount > 0}<span class="count"> · {activeCount}</span>{/if}
    </h3>
    {#if activeCount > 0 && onClear}
      <button type="button" class="clear" onclick={onClear}>
        Clear<span class="x" aria-hidden="true">×</span>
      </button>
    {/if}
  </header>
  <div class="body">
    {@render children()}
  </div>
</section>

<style>
  .filter-section {
    border: 1px solid var(--border-subtle);
    background: var(--bg-secondary);
    border-radius: var(--radius);
    padding: var(--space-4);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    margin-bottom: var(--space-3);
  }
  .title {
    margin: 0;
    font-family: var(--font-serif);
    font-size: var(--text-body-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    letter-spacing: var(--tracking-tight);
  }
  .count {
    color: var(--accent);
    font-weight: var(--font-weight-semibold);
  }

  .clear {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    background: transparent;
    border: none;
    padding: 0;
    cursor: pointer;
    color: var(--text-muted);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .clear:hover { color: var(--accent); }
  .clear .x { font-size: 1rem; line-height: 1; }

  .body { display: flex; flex-direction: column; gap: var(--space-2); }
</style>
