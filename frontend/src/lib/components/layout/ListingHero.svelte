<script lang="ts">
  import type { Snippet } from 'svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import ExpandableDescription from '$lib/components/common/ExpandableDescription.svelte';

  let {
    eyebrow,
    title,
    subtitle,
    description,
  }: {
    eyebrow?: string;
    title: string;
    subtitle?: string;
    /** Renders an expandable description on the right column when supplied. */
    description?: Snippet;
  } = $props();
</script>

<div class="hero-band">
  <div class="hero-inner">
    <div class="hero-title-col">
      {#if eyebrow}
        <div class="eyebrow"><Eyebrow>{eyebrow}</Eyebrow></div>
      {/if}
      <h1 class="hero-title">{title}</h1>
      {#if subtitle}<p class="hero-subtitle">{subtitle}</p>{/if}
    </div>
    {#if description}
      <div class="hero-desc-col">
        <ExpandableDescription>
          {@render description()}
        </ExpandableDescription>
      </div>
    {/if}
  </div>
</div>

<style>
  /* Full-bleed warm-gold panel — breaks out of any max-width container. */
  .hero-band {
    width: 100vw;
    margin-left: calc(50% - 50vw);
    margin-right: calc(50% - 50vw);
    background: var(--hero-bg);
    border-bottom: 1px solid var(--border-subtle);
    min-height: 240px;
    padding: var(--space-10) 0 var(--space-8);
  }
  @media (min-width: 768px) {
    .hero-band {
      min-height: 300px;
      padding: var(--space-12) 0 var(--space-10);
    }
  }

  .hero-inner {
    max-width: var(--page-width);
    margin: 0 auto;
    padding: 0 var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }
  @media (min-width: 768px) {
    .hero-inner {
      flex-direction: row;
      align-items: flex-start;
      gap: var(--space-10);
    }
  }

  .hero-title-col { flex: 1 1 33%; min-width: 0; }
  .hero-desc-col  { flex: 1 1 67%; min-width: 0; }
  @media (min-width: 768px) {
    .hero-desc-col { padding-top: var(--space-3); }
  }

  .eyebrow { margin-bottom: var(--space-2); }

  .hero-title {
    font-family: var(--font-serif);
    font-size: clamp(2.5rem, 6vw, 4.5rem);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-tight);
    line-height: 1.05;
    color: var(--hero-fg, var(--text-primary));
    margin: var(--space-2) 0 0;
  }
  .hero-subtitle {
    margin: var(--space-4) 0 0;
    color: var(--hero-subtle-fg, var(--accent));
    font-family: var(--font-serif);
    font-size: var(--text-lead);
    line-height: 1.4;
  }

  @media (max-width: 640px) {
    .hero-band { padding: var(--space-8) 0; min-height: 200px; }
    .hero-title { font-size: clamp(2rem, 8vw, 3rem); }
  }
</style>
