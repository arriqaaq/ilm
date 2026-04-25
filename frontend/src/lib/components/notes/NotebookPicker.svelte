<script lang="ts">
  import type { Notebook } from '$lib/types';
  import { fetchNotebooks } from '$lib/api';

  let { value = $bindable(null), onchange }: {
    value: string | null;
    onchange?: (id: string | null) => void;
  } = $props();

  let notebooks: Notebook[] = $state([]);
  let open = $state(false);

  $effect(() => {
    fetchNotebooks().then(nbs => { notebooks = nbs; }).catch(() => {});
  });

  let rootNotebooks = $derived(notebooks.filter(n => !n.parent_id));

  function childrenOf(parentId: string): Notebook[] {
    return notebooks.filter(n => n.parent_id === parentId);
  }

  let selectedLabel = $derived.by(() => {
    if (!value) return 'No notebook';
    const nb = notebooks.find(n => n.id === value);
    return nb ? `${nb.emoji ?? ''} ${nb.name}`.trim() : 'No notebook';
  });

  function select(id: string | null) {
    value = id;
    open = false;
    onchange?.(id);
  }

  function handleClickOutside(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.notebook-picker')) {
      open = false;
    }
  }

  $effect(() => {
    if (open) {
      document.addEventListener('click', handleClickOutside, true);
      return () => document.removeEventListener('click', handleClickOutside, true);
    }
  });
</script>

<div class="notebook-picker">
  <button class="picker-trigger" onclick={() => { open = !open; }}>
    <span class="picker-label">{selectedLabel}</span>
    <span class="picker-arrow">&#9662;</span>
  </button>

  {#if open}
    <div class="picker-dropdown">
      <button class="picker-item" class:active={value === null} onclick={() => select(null)}>
        <span class="item-emoji">&#8212;</span>
        <span class="item-name">No notebook</span>
      </button>
      {#each rootNotebooks as nb (nb.id)}
        <button class="picker-item" class:active={value === nb.id} onclick={() => select(nb.id)}>
          <span class="item-emoji">{nb.emoji ?? '&#128193;'}</span>
          <span class="item-name">{nb.name}</span>
        </button>
        {#each childrenOf(nb.id) as child (child.id)}
          <button class="picker-item child" class:active={value === child.id} onclick={() => select(child.id)}>
            <span class="item-emoji">{child.emoji ?? '&#128193;'}</span>
            <span class="item-name">{child.name}</span>
          </button>
        {/each}
      {/each}
    </div>
  {/if}
</div>

<style>
  .notebook-picker {
    position: relative;
  }
  .picker-trigger {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 12px;
    font-size: 0.78rem;
    font-family: var(--font-sans);
    color: var(--text-secondary);
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    cursor: pointer;
    transition: all var(--transition);
    white-space: nowrap;
  }
  .picker-trigger:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .picker-label {
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .picker-arrow {
    font-size: 0.55rem;
    color: var(--text-muted);
  }

  .picker-dropdown {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    z-index: 50;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: 0 8px 32px -8px rgba(0,0,0,0.18), 0 0 0 1px rgba(218,221,227,0.2);
    min-width: 180px;
    max-height: 240px;
    overflow-y: auto;
    animation: popIn 0.12s ease-out;
  }
  .picker-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 8px 14px;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: 0.8rem;
    font-family: var(--font-sans);
    cursor: pointer;
    text-align: left;
    transition: background var(--transition);
  }
  .picker-item:hover {
    background: var(--bg-hover);
  }
  .picker-item.active {
    color: var(--accent);
    font-weight: 600;
  }
  .picker-item.child {
    padding-left: 30px;
    font-size: 0.78rem;
  }
  .item-emoji {
    font-size: 0.9rem;
    width: 18px;
    text-align: center;
  }
  .item-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @keyframes popIn {
    from { opacity: 0; transform: translateY(-4px); }
    to { opacity: 1; transform: translateY(0); }
  }
</style>
