<script lang="ts">
  import type { Snippet } from 'svelte';

  let { activeTab = 'content', content, search, chat }: {
    activeTab?: 'content' | 'search' | 'chat';
    content: Snippet;
    search?: Snippet;
    chat: Snippet;
  } = $props();

  // svelte-ignore state_referenced_locally
  let currentTab: 'content' | 'search' | 'chat' = $state(activeTab);
</script>

<div class="sidebar-tabs-container">
  <div class="tab-bar" role="tablist">
    <button
      class="tab-btn"
      class:active={currentTab === 'content'}
      role="tab"
      aria-selected={currentTab === 'content'}
      aria-label="Contents"
      onclick={() => currentTab = 'content'}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/>
        <line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/>
      </svg>
    </button>
    {#if search}
      <button
        class="tab-btn"
        class:active={currentTab === 'search'}
        role="tab"
        aria-selected={currentTab === 'search'}
        aria-label="Search inside book"
        onclick={() => currentTab = 'search'}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
        </svg>
      </button>
    {/if}
    <button
      class="tab-btn"
      class:active={currentTab === 'chat'}
      role="tab"
      aria-selected={currentTab === 'chat'}
      aria-label="Ask AI"
      onclick={() => currentTab = 'chat'}
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5z"/>
        <path d="M19 13l.7 2.1L22 16l-2.3.9L19 19l-.7-2.1L16 16l2.3-.9z"/>
      </svg>
    </button>
  </div>

  <div class="tab-content" role="tabpanel">
    {#if currentTab === 'content'}
      {@render content()}
    {:else if currentTab === 'search' && search}
      {@render search()}
    {:else}
      {@render chat()}
    {/if}
  </div>
</div>

<style>
  .sidebar-tabs-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .tab-bar {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--border-subtle);
    flex-shrink: 0;
  }

  .tab-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 44px;
    border: none;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    transition: all var(--transition);
  }

  .tab-btn:hover {
    color: var(--text-secondary);
    background: var(--bg-hover);
  }

  .tab-btn.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }

  .tab-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
