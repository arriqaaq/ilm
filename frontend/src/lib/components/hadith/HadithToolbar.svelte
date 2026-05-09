<script lang="ts">
  import Icon from '$lib/components/common/Icon.svelte';

  let {
    q,
    sort,
    view,
    onChange,
  }: {
    q: string;
    sort: 'number_asc' | 'number_desc';
    view: 'list' | 'grid';
    onChange: (patch: {
      q?: string;
      sort?: 'number_asc' | 'number_desc';
      view?: 'list' | 'grid';
    }) => void;
  } = $props();

  let qInput: string = $state('');
  let qTimer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => { qInput = q; });

  function handleQ(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    qInput = value;
    if (qTimer) clearTimeout(qTimer);
    qTimer = setTimeout(() => onChange({ q: value }), 400);
  }
</script>

<div class="toolbar">
  <label class="search">
    <span class="search-icon"><Icon name="search" size="sm" /></span>
    <input
      type="text"
      placeholder="Search in hadiths…"
      value={qInput}
      oninput={handleQ}
    />
  </label>

  <div class="trail">
    <select
      class="sort"
      value={sort}
      onchange={(e) => onChange({ sort: (e.target as HTMLSelectElement).value as 'number_asc' | 'number_desc' })}
      aria-label="Sort"
    >
      <option value="number_asc">Number ↑</option>
      <option value="number_desc">Number ↓</option>
    </select>

    <div class="view-switch" role="group" aria-label="View">
      <button
        type="button"
        class:active={view === 'list'}
        onclick={() => onChange({ view: 'list' })}
        aria-label="List view"
        aria-pressed={view === 'list'}
      >
        <Icon name="list" size="sm" />
      </button>
      <button
        type="button"
        class:active={view === 'grid'}
        onclick={() => onChange({ view: 'grid' })}
        aria-label="Grid view"
        aria-pressed={view === 'grid'}
      >
        <Icon name="grid" size="sm" />
      </button>
    </div>
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding-bottom: var(--space-5);
    margin-bottom: var(--space-5);
    border-bottom: 1px solid var(--border-subtle);
    flex-wrap: wrap;
  }

  .search {
    position: relative;
    flex: 1 1 100%;
    min-width: 220px;
    display: block;
    order: 0;
  }
  @media (min-width: 640px) {
    .search { flex: 1 1 0; }
  }
  .search-icon {
    position: absolute;
    left: var(--space-3);
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-muted);
    pointer-events: none;
  }
  .search input {
    width: 100%;
    padding: var(--space-3) var(--space-3) var(--space-3) calc(var(--space-3) * 2 + 16px);
    font-family: var(--font-sans);
    font-size: var(--text-body);
    color: var(--text-primary);
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    transition: border-color var(--transition), box-shadow var(--transition);
  }
  .search input:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-muted);
  }

  .trail {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .sort {
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-primary);
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
  }
  .sort:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-muted);
  }

  .view-switch {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    background: var(--bg-surface);
  }
  .view-switch button {
    background: transparent;
    color: var(--text-muted);
    border: none;
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    transition: background var(--transition), color var(--transition);
  }
  .view-switch button + button {
    border-left: 1px solid var(--border);
  }
  .view-switch button:hover { color: var(--text-primary); background: var(--bg-hover); }
  .view-switch button.active {
    background: var(--accent-muted);
    color: var(--accent);
  }
</style>
