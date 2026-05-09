<script lang="ts">
  import { narratorAutocomplete } from '$lib/api';
  import type { ApiNarratorSearchResult } from '$lib/types';
  import FilterSection from './FilterSection.svelte';
  import Icon from '$lib/components/common/Icon.svelte';

  let {
    selected,
    selectedDetails,
    onChange,
  }: {
    /** Narrator slugs (record-id keys, no `narrator:` prefix). */
    selected: string[];
    /** Cached metadata for each selected id, so we can render names without re-querying. */
    selectedDetails: ApiNarratorSearchResult[];
    onChange: (next: { ids: string[]; details: ApiNarratorSearchResult[] }) => void;
  } = $props();

  let query = $state('');
  let suggestions: ApiNarratorSearchResult[] = $state([]);
  let loading = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function search(q: string) {
    if (timer) clearTimeout(timer);
    if (!q.trim()) { suggestions = []; loading = false; return; }
    loading = true;
    timer = setTimeout(async () => {
      try {
        suggestions = await narratorAutocomplete(q.trim(), 10);
      } catch (e) {
        suggestions = [];
      } finally {
        loading = false;
      }
    }, 250);
  }

  $effect(() => { search(query); });

  function selectedSet(): Set<string> { return new Set(selected); }

  function toggle(n: ApiNarratorSearchResult) {
    const set = selectedSet();
    if (set.has(n.id)) {
      const ids = selected.filter(id => id !== n.id);
      const details = selectedDetails.filter(d => d.id !== n.id);
      onChange({ ids, details });
    } else {
      const ids = [...selected, n.id];
      const details = [...selectedDetails.filter(d => d.id !== n.id), n];
      onChange({ ids, details });
    }
  }

  // Hide suggestions that are already selected (they show in the pinned list).
  const visibleSuggestions = $derived(
    suggestions.filter(s => !selected.includes(s.id))
  );
</script>

<FilterSection
  title="Narrator"
  activeCount={selected.length}
  onClear={() => onChange({ ids: [], details: [] })}
>
  <div class="search">
    <span class="search-icon"><Icon name="search" size="xs" /></span>
    <input
      type="text"
      placeholder="Search narrators…"
      bind:value={query}
    />
  </div>

  {#if selectedDetails.length > 0}
    <ul class="picked">
      {#each selectedDetails as n (n.id)}
        <li>
          <label class="opt picked-opt">
            <input type="checkbox" checked onchange={() => toggle(n)} />
            <span class="names">
              {#if n.name_ar}
                <span class="ar arabic-prose" dir="rtl">{n.name_ar}</span>
              {/if}
              {#if n.name_en}<span class="en">{n.name_en}</span>{/if}
            </span>
          </label>
        </li>
      {/each}
    </ul>
  {/if}

  {#if loading}
    <div class="hint">Searching…</div>
  {:else if visibleSuggestions.length > 0}
    <ul class="results">
      {#each visibleSuggestions as n (n.id)}
        <li>
          <label class="opt">
            <input
              type="checkbox"
              checked={false}
              onchange={() => toggle(n)}
            />
            <span class="names">
              {#if n.name_ar}
                <span class="ar arabic-prose" dir="rtl">{n.name_ar}</span>
              {/if}
              {#if n.name_en}<span class="en">{n.name_en}</span>{/if}
            </span>
            {#if n.hadith_count !== null}
              <span class="count">{n.hadith_count.toLocaleString()}</span>
            {/if}
          </label>
        </li>
      {/each}
    </ul>
  {:else if query.trim() && !loading}
    <div class="hint">No matches.</div>
  {/if}
</FilterSection>

<style>
  .search {
    position: relative;
    margin-bottom: var(--space-2);
  }
  .search-icon {
    position: absolute;
    left: var(--space-2);
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted);
    pointer-events: none;
  }
  .search input {
    width: 100%;
    padding: var(--space-2) var(--space-2) var(--space-2) calc(var(--space-2) * 2 + 14px);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-primary);
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .search input:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-muted);
  }

  .picked, .results {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 320px;
    overflow-y: auto;
  }
  .picked { margin-bottom: var(--space-2); }

  .opt {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: baseline;
    gap: var(--space-2);
    padding: var(--space-2);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background var(--transition);
  }
  .opt:hover { background: var(--bg-hover); }
  .picked-opt { background: var(--accent-muted); }

  .opt input[type='checkbox'] {
    accent-color: var(--accent);
    width: 14px;
    height: 14px;
    margin: 0;
    cursor: pointer;
  }

  .names {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .en {
    font-size: var(--text-meta);
    color: var(--text-primary);
    line-height: 1.3;
  }
  .ar {
    font-size: var(--text-meta);
    color: var(--text-secondary);
    line-height: 1.4;
  }
  .count {
    font-family: var(--font-mono);
    font-size: var(--text-2xs);
    color: var(--text-muted);
  }

  .hint {
    font-size: var(--text-2xs);
    color: var(--text-muted);
    padding: var(--space-2);
    text-align: center;
  }
</style>
