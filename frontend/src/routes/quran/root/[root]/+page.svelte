<script lang="ts">
  import { page } from '$app/stores';
  import { searchByRoot } from '$lib/api';
  import type { RootSearchResponse } from '$lib/types';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import MetaRow from '$lib/components/common/MetaRow.svelte';
  import SectionHeading from '$lib/components/common/SectionHeading.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';

  let data: RootSearchResponse | null = $state(null);
  let loading = $state(true);
  let error: string | null = $state(null);

  let root = $derived(($page.params as Record<string, string>).root);

  $effect(() => {
    if (root) {
      loading = true;
      error = null;
      searchByRoot(root)
        .then(r => { data = r; })
        .catch(e => { error = e.message; })
        .finally(() => { loading = false; });
    }
  });

  // Group occurrences by surah:ayah
  let grouped = $derived.by(() => {
    if (!data) return [];
    const map = new Map<string, typeof data.occurrences>();
    for (const w of data.occurrences) {
      const key = `${w.surah_number}:${w.ayah_number}`;
      if (!map.has(key)) map.set(key, []);
      map.get(key)!.push(w);
    }
    return Array.from(map.entries()).map(([key, words]) => ({ key, words }));
  });
</script>

<svelte:head>
  <title>Root: {root} — Qurʾān</title>
</svelte:head>

<div class="page-shell-narrow">
  <header class="root-header">
    <Eyebrow>QURʾĀN · ROOT</Eyebrow>
    <h1 class="root-title arabic-prose" dir="rtl">{root}</h1>
    {#if data}
      <MetaRow items={[
        `${data.occurrences.length} occurrences`,
        `${data.ayah_count} āyāt`,
      ]} />
    {/if}
  </header>

  <hr class="separator" />

  {#if loading}
    <div class="state"><LoadingSpinner /></div>
  {:else if error}
    <div class="state error">{error}</div>
  {:else if grouped.length === 0}
    <div class="state">No occurrences found for this root.</div>
  {:else}
    <SectionHeading eyebrow="Occurrences" title="Where this root appears" level={2} />
    <div class="root-results">
      {#each grouped as { key, words }}
        <article class="root-ayah">
          <a href="/quran/{words[0].surah_number}#{key}" class="ayah-link mono">{key}</a>
          <div class="word-list" dir="rtl">
            {#each words as word}
              <span class="root-word">
                <span class="rw-ar">{word.text_ar}</span>
                {#if word.translation}
                  <span class="rw-en">{word.translation}</span>
                {/if}
                <span class="rw-pos">{word.pos}</span>
              </span>
            {/each}
          </div>
        </article>
      {/each}
    </div>
  {/if}
</div>

<style>
  .root-header { text-align: center; margin-bottom: var(--space-3); }
  .root-title {
    font-size: clamp(3rem, 8vw, 5rem);
    color: var(--text-primary);
    line-height: 1.4;
    margin: var(--space-3) 0 var(--space-3);
    font-weight: var(--font-weight-semibold);
    letter-spacing: 0.1em;
  }

  .separator {
    border: none;
    border-top: 1px solid var(--border-subtle);
    margin: var(--space-6) 0;
  }

  .state {
    padding: var(--space-12);
    text-align: center;
    color: var(--text-muted);
    font-family: var(--font-serif);
    font-style: italic;
  }
  .state.error { color: var(--error); }

  .root-results {
    display: flex;
    flex-direction: column;
  }
  .root-ayah {
    display: flex;
    align-items: flex-start;
    gap: var(--space-4);
    padding: var(--space-4) 0;
    border-bottom: 1px solid var(--border-subtle);
  }
  .root-ayah:last-child { border-bottom: none; }

  .ayah-link {
    flex-shrink: 0;
    font-size: var(--text-meta);
    color: var(--accent);
    font-weight: var(--font-weight-semibold);
    text-decoration: none;
    min-width: 56px;
    line-height: 1.5;
  }
  .ayah-link:hover { text-decoration: underline; }

  .word-list {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .root-word {
    display: inline-flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    padding: var(--space-2) var(--space-3);
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
  }
  .rw-ar {
    font-size: 1.3rem;
    color: var(--text-primary);
  }
  .rw-en {
    font-family: var(--font-serif);
    font-size: var(--text-2xs);
    color: var(--text-muted);
    font-style: italic;
  }
  .rw-pos {
    font-size: var(--text-2xs);
    color: var(--accent);
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
</style>
