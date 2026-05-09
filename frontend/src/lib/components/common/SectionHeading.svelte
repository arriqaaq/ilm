<script lang="ts">
  import Eyebrow from './Eyebrow.svelte';
  let { eyebrow, title, subtitle, level = 2 }: {
    eyebrow?: string;
    title: string;
    subtitle?: string;
    level?: 1 | 2 | 3;
  } = $props();
</script>

<div class="section-heading">
  {#if eyebrow}
    <div class="eyebrow-row"><Eyebrow>{eyebrow}</Eyebrow></div>
  {/if}
  {#if level === 1}
    <h1 class="title">{title}</h1>
  {:else if level === 2}
    <h2 class="title">{title}</h2>
  {:else}
    <h3 class="title">{title}</h3>
  {/if}
  {#if subtitle}
    <p class="subtitle">{subtitle}</p>
  {/if}
</div>

<style>
  .section-heading { margin-bottom: var(--space-5); }
  .eyebrow-row { margin-bottom: var(--space-2); }
  .title {
    font-family: var(--font-serif);
    color: var(--text-primary);
    font-weight: var(--font-weight-semibold);
    line-height: var(--leading-tight);
    letter-spacing: var(--tracking-tight);
    margin: 0;
  }
  h1.title { font-size: 2.1rem; }
  h2.title { font-size: var(--text-xl); }
  h3.title { font-size: var(--text-lg); }
  .subtitle {
    margin: var(--space-2) 0 0;
    font-family: var(--font-serif);
    font-size: var(--text-body);
    color: var(--text-secondary);
    font-style: italic;
  }
</style>
