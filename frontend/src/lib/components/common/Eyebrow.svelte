<script lang="ts">
  import type { Snippet } from 'svelte';
  let { children, tone = 'accent' }: {
    children: Snippet;
    tone?: 'accent' | 'muted';
  } = $props();
</script>

<span class="eyebrow tone-{tone}">{@render children()}</span>

<style>
  .eyebrow {
    display: inline-block;
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    line-height: 1;
  }
  .tone-accent { color: var(--accent); }
  .tone-muted  { color: var(--text-muted); }
</style>
