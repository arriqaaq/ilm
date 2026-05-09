<script lang="ts">
  import Ornament from './Ornament.svelte';
  let { variant = 'hairline' }: { variant?: 'hairline' | 'ornamental' } = $props();
</script>

{#if variant === 'ornamental'}
  <div class="ornamental">
    <Ornament variant="divider" size={14} color="var(--accent)" />
  </div>
{:else}
  <hr class="hairline" />
{/if}

<style>
  .hairline {
    border: none;
    border-top: 1px solid var(--border-subtle);
    margin: var(--space-6) 0;
  }
  .ornamental {
    margin: var(--space-8) 0;
    opacity: 0.7;
  }
</style>
