<script lang="ts">
  import type { Notebook } from '$lib/types';
  import { fetchNotebooks, createNotebook, updateNotebook, deleteNotebook } from '$lib/api';

  let { activeNotebookId = $bindable(null) }: {
    activeNotebookId: string | null;
  } = $props();

  let notebooks: Notebook[] = $state([]);
  let showNewForm = $state(false);
  let newName = $state('');
  let newEmoji = $state('');
  let newParentId: string | null = $state(null);
  let editingId: string | null = $state(null);
  let editName = $state('');

  $effect(() => {
    fetchNotebooks().then(nbs => { notebooks = nbs; }).catch(() => {});
  });

  let rootNotebooks = $derived(notebooks.filter(n => !n.parent_id));

  function childrenOf(parentId: string): Notebook[] {
    return notebooks.filter(n => n.parent_id === parentId);
  }

  async function handleCreate() {
    if (!newName.trim()) return;
    const nb = await createNotebook({
      name: newName.trim(),
      emoji: newEmoji.trim() || undefined,
      parent_id: newParentId || undefined,
    });
    notebooks = [...notebooks, nb];
    newName = '';
    newEmoji = '';
    newParentId = null;
    showNewForm = false;
  }

  async function handleRename(id: string) {
    if (!editName.trim()) return;
    const updated = await updateNotebook(id, { name: editName.trim() });
    notebooks = notebooks.map(n => n.id === id ? updated : n);
    editingId = null;
  }

  async function handleDelete(id: string) {
    await deleteNotebook(id);
    notebooks = notebooks.filter(n => n.id !== id && n.parent_id !== id);
    if (activeNotebookId === id) activeNotebookId = null;
  }

  function selectNotebook(id: string | null) {
    activeNotebookId = activeNotebookId === id ? null : id;
  }
</script>

<div class="notebook-sidebar">
  <div class="sidebar-header">
    <span class="sidebar-title">NOTEBOOKS</span>
    <button class="add-btn" onclick={() => { showNewForm = !showNewForm; }} title="New notebook">+</button>
  </div>

  {#if showNewForm}
    <div class="new-form">
      <div class="new-row">
        <input
          class="emoji-input"
          placeholder="Icon"
          bind:value={newEmoji}
          maxlength="2"
        />
        <input
          class="name-input"
          placeholder="Notebook name"
          bind:value={newName}
          onkeydown={(e) => { if (e.key === 'Enter') handleCreate(); }}
        />
      </div>
      {#if notebooks.length > 0}
        <select class="parent-select" bind:value={newParentId}>
          <option value={null}>No parent (root)</option>
          {#each rootNotebooks as nb}
            <option value={nb.id}>{nb.emoji ?? ''} {nb.name}</option>
          {/each}
        </select>
      {/if}
      <div class="form-actions">
        <button class="create-btn" onclick={handleCreate}>Create</button>
        <button class="cancel-btn" onclick={() => { showNewForm = false; }}>Cancel</button>
      </div>
    </div>
  {/if}

  <div class="nb-list">
    <!-- All Notes -->
    <button
      class="nb-item"
      class:active={activeNotebookId === null}
      onclick={() => selectNotebook(null)}
    >
      <span class="nb-emoji">&#128209;</span>
      <span class="nb-name">All Notes</span>
    </button>

    <!-- Uncategorized -->
    <button
      class="nb-item"
      class:active={activeNotebookId === '__uncategorized__'}
      onclick={() => selectNotebook('__uncategorized__')}
    >
      <span class="nb-emoji">&#128196;</span>
      <span class="nb-name">Uncategorized</span>
    </button>

    {#each rootNotebooks as nb (nb.id)}
      {@const children = childrenOf(nb.id)}

      {#if editingId === nb.id}
        <div class="nb-edit-row">
          <input
            class="edit-input"
            bind:value={editName}
            onkeydown={(e) => { if (e.key === 'Enter') handleRename(nb.id); if (e.key === 'Escape') editingId = null; }}
          />
          <button class="save-btn" onclick={() => handleRename(nb.id)}>&#10003;</button>
        </div>
      {:else}
        <button
          class="nb-item"
          class:active={activeNotebookId === nb.id}
          onclick={() => selectNotebook(nb.id)}
        >
          <span class="nb-emoji">{nb.emoji ?? '📁'}</span>
          <span class="nb-name">{nb.name}</span>
          <span class="nb-actions">
            <span class="action" role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); editingId = nb.id; editName = nb.name; }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); editingId = nb.id; editName = nb.name; } }} title="Rename">&#9998;</span>
            <span class="action delete" role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); handleDelete(nb.id); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); handleDelete(nb.id); } }} title="Delete">&times;</span>
          </span>
        </button>
      {/if}

      {#each children as child (child.id)}
        {#if editingId === child.id}
          <div class="nb-edit-row child">
            <input
              class="edit-input"
              bind:value={editName}
              onkeydown={(e) => { if (e.key === 'Enter') handleRename(child.id); if (e.key === 'Escape') editingId = null; }}
            />
            <button class="save-btn" onclick={() => handleRename(child.id)}>&#10003;</button>
          </div>
        {:else}
          <button
            class="nb-item child"
            class:active={activeNotebookId === child.id}
            onclick={() => selectNotebook(child.id)}
          >
            <span class="nb-emoji">{child.emoji ?? '📁'}</span>
            <span class="nb-name">{child.name}</span>
            <span class="nb-actions">
              <span class="action" role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); editingId = child.id; editName = child.name; }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); editingId = child.id; editName = child.name; } }} title="Rename">&#9998;</span>
              <span class="action delete" role="button" tabindex="0" onclick={(e) => { e.stopPropagation(); handleDelete(child.id); }} onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); handleDelete(child.id); } }} title="Delete">&times;</span>
            </span>
          </button>
        {/if}
      {/each}
    {/each}
  </div>
</div>

<style>
  .notebook-sidebar {
    width: 220px;
    flex-shrink: 0;
    border-right: 1px solid var(--border-subtle);
    padding: 20px 0;
    overflow-y: auto;
  }
  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    margin-bottom: 12px;
  }
  .sidebar-title {
    font-size: 0.68rem;
    font-weight: 700;
    letter-spacing: 1.2px;
    color: var(--text-muted);
    text-transform: uppercase;
  }
  .add-btn {
    width: 24px;
    height: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 1rem;
    transition: all var(--transition);
  }
  .add-btn:hover {
    background: var(--bg-hover);
    color: var(--accent);
    border-color: var(--accent);
  }

  .new-form {
    padding: 8px 16px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .new-row {
    display: flex;
    gap: 6px;
  }
  .emoji-input {
    width: 40px;
    padding: 6px 8px;
    text-align: center;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.9rem;
    outline: none;
  }
  .name-input {
    flex: 1;
    padding: 6px 10px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.8rem;
    font-family: var(--font-serif);
    outline: none;
  }
  .name-input:focus, .emoji-input:focus {
    border-color: var(--accent);
  }
  .parent-select {
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.75rem;
    outline: none;
  }
  .form-actions {
    display: flex;
    gap: 6px;
  }
  .create-btn {
    padding: 5px 14px;
    font-size: var(--text-2xs);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: var(--btn-letter-spacing);
    background: var(--accent);
    color: var(--btn-primary-fg);
    border: none;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
  .cancel-btn {
    padding: 5px 10px;
    font-size: 0.72rem;
    background: none;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-muted);
    cursor: pointer;
  }

  .nb-list {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .nb-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 16px;
    border: none;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition);
    text-align: left;
    width: 100%;
    font-size: 0.85rem;
    font-family: var(--font-sans);
    position: relative;
  }
  .nb-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .nb-item.active {
    background: var(--accent-muted);
    color: var(--accent);
    font-weight: 600;
  }
  .nb-item.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 4px;
    bottom: 4px;
    width: 3px;
    border-radius: 0 3px 3px 0;
    background: var(--accent);
  }
  .nb-item.child {
    padding-left: 36px;
    font-size: 0.82rem;
  }
  .nb-emoji {
    font-size: 1rem;
    width: 20px;
    text-align: center;
    flex-shrink: 0;
  }
  .nb-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .nb-actions {
    display: none;
    gap: 2px;
    flex-shrink: 0;
  }
  .nb-item:hover .nb-actions {
    display: flex;
  }
  .action {
    background: none;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.75rem;
    padding: 2px 4px;
    border-radius: 4px;
    transition: all var(--transition);
  }
  .action:hover {
    color: var(--text-primary);
    background: var(--bg-active);
  }
  .action.delete:hover {
    color: var(--error);
    background: rgba(220,38,38,0.08);
  }

  .nb-edit-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 16px;
  }
  .nb-edit-row.child {
    padding-left: 36px;
  }
  .edit-input {
    flex: 1;
    padding: 4px 8px;
    border: 1px solid var(--accent);
    border-radius: 6px;
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: 0.8rem;
    outline: none;
  }
  .save-btn {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font-size: 0.85rem;
  }

  @media (max-width: 768px) {
    .notebook-sidebar {
      width: 100%;
      border-right: none;
      border-bottom: 1px solid var(--border-subtle);
      padding: 12px 0;
    }
    .nb-list {
      flex-direction: row;
      flex-wrap: wrap;
      gap: 4px;
      padding: 0 12px;
    }
    .nb-item {
      padding: 6px 12px;
      border-radius: 20px;
      border: 1px solid var(--border-subtle);
      width: auto;
    }
    .nb-item.active {
      border-color: var(--accent);
    }
    .nb-item.active::before {
      display: none;
    }
    .nb-item.child {
      padding-left: 12px;
    }
  }
</style>
