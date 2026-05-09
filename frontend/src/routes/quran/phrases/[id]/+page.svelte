<script lang="ts">
  import { page } from '$app/stores';
  import { getPhraseDetail } from '$lib/api';
  import type { ApiPhraseWithAyahs } from '$lib/types';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import MetaRow from '$lib/components/common/MetaRow.svelte';
  import SectionHeading from '$lib/components/common/SectionHeading.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let data: ApiPhraseWithAyahs | null = $state(null);
  let loading = $state(true);
  let error: string | null = $state(null);

  let phraseId = $derived(($page.params as Record<string, string>).id);

  $effect(() => {
    if (phraseId) {
      loading = true;
      error = null;
      getPhraseDetail(phraseId)
        .then(r => { data = r; })
        .catch(e => { error = e.message; })
        .finally(() => { loading = false; });
    }
  });
</script>

<svelte:head>
  <title>{data ? `${data.text_ar} · Phrase` : 'Phrase'} — Ilm</title>
</svelte:head>

<div class="page-shell-narrow">
  {#if loading}
    <div class="state"><LoadingSpinner /></div>
  {:else if error}
    <div class="state error">{error}</div>
  {:else if data}
    <header class="phrase-header">
      <Eyebrow>QURʾĀN · PHRASE</Eyebrow>
      <h1 class="phrase-title arabic-prose" dir="rtl">{data.text_ar}</h1>
      <MetaRow items={[
        `${data.occurrence} occurrences`,
        `${data.ayah_keys.length} āyāt`,
      ]} />
    </header>

    <hr class="separator" />

    {#if data.ayah_keys.length === 0}
      <div class="state">No āyāt found for this phrase.</div>
    {:else}
      <SectionHeading eyebrow="Occurrences" title="Where this phrase appears" level={2} />
      <div class="ayah-list">
        {#each data.ayah_keys as key}
          {@const parts = key.split(':')}
          <a href="/quran/{parts[0]}?ayah={parts[1]}" class="ayah-chip">
            <span class="ayah-ref mono">{key}</span>
          </a>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .state {
    padding: var(--space-12);
    text-align: center;
    color: var(--text-muted);
    font-family: var(--font-serif);
    font-style: italic;
  }
  .state.error { color: var(--error); }

  .phrase-header { text-align: center; margin-bottom: var(--space-3); }
  .phrase-title {
    font-size: clamp(2rem, 5vw, 2.8rem);
    color: var(--text-primary);
    line-height: 1.6;
    margin: var(--space-3) 0 var(--space-3);
    font-weight: var(--font-weight-semibold);
  }

  .separator {
    border: none;
    border-top: 1px solid var(--border-subtle);
    margin: var(--space-6) 0;
  }

  .ayah-list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .ayah-chip {
    display: inline-flex;
    align-items: center;
    padding: var(--space-2) var(--space-3);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    text-decoration: none;
    transition: all var(--transition);
  }
  .ayah-chip:hover {
    border-color: var(--accent);
    background: var(--accent-muted);
  }
  .ayah-ref {
    font-size: var(--text-meta);
    color: var(--accent);
    font-weight: var(--font-weight-semibold);
  }
</style>
