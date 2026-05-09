<script lang="ts">
  import Eyebrow from './Eyebrow.svelte';
  import MetaRow from './MetaRow.svelte';
  let { eyebrow, title, subtitle, meta }: {
    eyebrow?: string;
    title: string;
    subtitle?: string;
    meta?: (string | null | undefined)[];
  } = $props();
</script>

<header class="page-header">
  {#if eyebrow}<div class="eb"><Eyebrow>{eyebrow}</Eyebrow></div>{/if}
  <h1 class="title">{title}</h1>
  {#if subtitle}<p class="subtitle">{subtitle}</p>{/if}
  {#if meta && meta.length > 0}<div class="meta-wrap"><MetaRow items={meta} /></div>{/if}
</header>

<style>
  .page-header { margin-bottom: var(--space-6); }
  .eb { margin-bottom: var(--space-2); }
  .title {
    font-family: var(--font-serif);
    font-size: 2.1rem;
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-tight);
    margin: var(--space-2) 0 0;
    line-height: 1.15;
  }
  .subtitle {
    margin: var(--space-2) 0 0;
    font-family: var(--font-serif);
    font-style: italic;
    color: var(--text-secondary);
    font-size: var(--text-body);
  }
  .meta-wrap { margin-top: var(--space-3); }
</style>
