<script lang="ts">
  import type { ApiCollection } from '$lib/types';
  import FilterSection from './FilterSection.svelte';

  let {
    collections,
    selected,
    onChange,
  }: {
    collections: ApiCollection[];
    selected: number[];
    onChange: (next: number[]) => void;
  } = $props();

  function toggle(id: number) {
    onChange(
      selected.includes(id) ? selected.filter(s => s !== id) : [...selected, id]
    );
  }
</script>

<FilterSection
  title="Collection"
  activeCount={selected.length}
  onClear={() => onChange([])}
>
  {#each collections as c (c.id)}
    {@const checked = selected.includes(c.collection_id)}
    <label class="row" class:checked>
      <input
        type="checkbox"
        {checked}
        onchange={() => toggle(c.collection_id)}
      />
      <span class="num mono">{c.collection_id}</span>
      <span class="names">
        {#if c.name_ar}
          <span class="ar arabic-prose" dir="rtl">{c.name_ar}</span>
        {/if}
        {#if c.name_en && c.name_en !== c.name_ar}
          <span class="en">{c.name_en}</span>
        {/if}
      </span>
    </label>
  {/each}
</FilterSection>

<style>
  .row {
    display: grid;
    grid-template-columns: auto 1.5rem 1fr;
    align-items: baseline;
    gap: var(--space-2);
    padding: var(--space-2);
    border-radius: var(--radius-sm);
    cursor: pointer;
    transition: background var(--transition);
  }
  .row:hover { background: var(--bg-hover); }
  .row.checked { background: var(--accent-muted); }

  .row input[type='checkbox'] {
    accent-color: var(--accent);
    width: 14px;
    height: 14px;
    margin: 0;
    cursor: pointer;
  }

  .num {
    font-size: var(--text-2xs);
    color: var(--text-muted);
    text-align: center;
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
</style>
