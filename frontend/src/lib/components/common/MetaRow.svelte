<script lang="ts">
  let { items }: { items: (string | null | undefined)[] } = $props();
  let visible = $derived(items.filter((s): s is string => !!s && s.trim().length > 0));
</script>

{#if visible.length > 0}
  <div class="meta-row">
    {#each visible as item, i}
      <span class="meta-item">{item}</span>
      {#if i < visible.length - 1}
        <span class="dot" aria-hidden="true">·</span>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .meta-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-secondary);
    line-height: 1.4;
  }
  .dot { color: var(--text-muted); }
</style>
