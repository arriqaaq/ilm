<script lang="ts">
  import { goto } from '$app/navigation';
  import LanguageToggle from './LanguageToggle.svelte';
  import QuranSettings from './QuranSettings.svelte';

  let { onToggleSidebar }: { onToggleSidebar?: () => void } = $props();

  let searchQuery = $state('');

  function handleSearch(e: Event) {
    e.preventDefault();
    if (searchQuery.trim()) {
      goto(`/explore?q=${encodeURIComponent(searchQuery.trim())}`);
    }
  }
</script>

<header class="topbar">
  <div class="topbar-left">
    {#if onToggleSidebar}
      <button class="hamburger" onclick={onToggleSidebar} aria-label="Menu">
        <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <line x1="3" y1="6" x2="21" y2="6"/>
          <line x1="3" y1="12" x2="21" y2="12"/>
          <line x1="3" y1="18" x2="21" y2="18"/>
        </svg>
      </button>
    {/if}
  </div>

  <div class="topbar-right">
    <form class="search-form" onsubmit={handleSearch}>
      <span class="search-icon">&#x2315;</span>
      <input
        type="text"
        placeholder="Search Quran & Sunnah..."
        bind:value={searchQuery}
        class="search-input"
      />
    </form>
    <QuranSettings />
    <LanguageToggle />
  </div>
</header>

<style>
  .topbar {
    height: var(--topbar-height);
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--accent-muted);
    box-shadow: 0 1px 0 var(--border-subtle);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 var(--space-5);
    flex-shrink: 0;
    gap: var(--space-3);
  }

  .topbar-left {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    flex-shrink: 0;
  }

  .hamburger {
    display: none;
    align-items: center;
    justify-content: center;
    width: var(--btn-height-md);
    height: var(--btn-height-md);
    border-radius: var(--radius);
    color: var(--text-secondary);
    background: none;
    border: none;
    cursor: pointer;
    transition: all var(--transition);
  }
  .hamburger:hover {
    color: var(--accent);
    background: var(--accent-muted);
  }

  .topbar-right {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-width: 0;
    flex: 1;
    justify-content: flex-end;
  }

  .search-form {
    display: flex;
    align-items: center;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 0 14px;
    flex: 1;
    max-width: 360px;
    min-width: 0;
    transition: all var(--transition);
  }

  .search-form:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-muted);
  }

  .search-icon {
    color: var(--accent);
    font-size: var(--text-base);
    margin-right: var(--space-2);
    flex-shrink: 0;
    opacity: 0.85;
  }

  .search-input {
    border: none;
    background: transparent;
    padding: var(--space-2) 0;
    width: 100%;
    font-size: var(--text-sm);
    font-family: var(--font-serif);
    min-width: 0;
  }
  .search-input::placeholder {
    font-style: italic;
    color: var(--text-muted);
  }

  .search-input:focus {
    border-color: transparent;
  }

  @media (max-width: 768px) {
    .topbar { padding: 0 var(--space-3); }
    .hamburger { display: flex; }
    .search-form { display: none; }
  }
</style>
