<script lang="ts" generics="T extends string">
  let {
    tabs,
    active = $bindable(),
    ariaLabel = 'Sections',
  }: {
    tabs: { id: T; label: string; count?: number; disabled?: boolean }[];
    active: T;
    ariaLabel?: string;
  } = $props();
</script>

<div class="tabs" role="tablist" aria-label={ariaLabel}>
  {#each tabs as t (t.id)}
    <button
      type="button"
      role="tab"
      class="tab"
      class:active={active === t.id}
      aria-selected={active === t.id}
      disabled={t.disabled}
      onclick={() => (active = t.id)}
    >
      <span class="label">{t.label}</span>
      {#if t.count !== undefined}<span class="count">({t.count.toLocaleString()})</span>{/if}
    </button>
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    gap: var(--space-2);
    border-bottom: 1px solid var(--border-subtle);
    overflow-x: auto;
    scrollbar-width: none;
    -webkit-overflow-scrolling: touch;
  }
  .tabs::-webkit-scrollbar { display: none; }

  .tab {
    flex-shrink: 0;
    white-space: nowrap;
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-1);
    padding: var(--space-3) var(--space-4);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-medium);
    color: var(--text-secondary);
    border: none;
    border-bottom: 2px solid transparent;
    background: transparent;
    margin-bottom: -1px;
    cursor: pointer;
    transition: color var(--transition), border-color var(--transition);
  }
  .tab:hover:not(:disabled) { color: var(--text-primary); }
  .tab.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .tab:disabled {
    color: var(--text-muted);
    cursor: not-allowed;
    opacity: 0.6;
  }
  .count {
    color: var(--text-muted);
    font-weight: var(--font-weight-medium);
  }
  .tab.active .count { color: var(--accent); opacity: 0.7; }
</style>
