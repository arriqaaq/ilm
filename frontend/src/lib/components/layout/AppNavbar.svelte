<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { appConfig } from '$lib/stores/config';
  import LanguageToggle from './LanguageToggle.svelte';
  import QuranSettings from './QuranSettings.svelte';
  import BrowseMenu from './BrowseMenu.svelte';

  let { variant = 'default' }: { variant?: 'default' | 'home' } = $props();

  let searchQuery = $state('');
  let mobileMenuOpen = $state(false);

  function handleSearch(e: Event) {
    e.preventDefault();
    if (!searchQuery.trim()) return;
    const enc = encodeURIComponent(searchQuery.trim());
    if ($appConfig.advanced_enabled) {
      goto(`/explore?q=${enc}&type=semantic`);
    } else {
      goto(`/search?q=${enc}&type=text`);
    }
  }

  let isHome = $derived(variant === 'home');
</script>

<header class="navbar" class:home={isHome}>
  <div class="navbar-inner">
    <a class="logo" href="/" aria-label="Ilm — Home">
      <span class="logo-glyph">❋</span>
      <span class="logo-en">Ilm</span>
      <span class="logo-ar" dir="rtl">عِلْم</span>
    </a>

    <nav class="primary" aria-label="Primary">
      <BrowseMenu />
      <a class="nav-link" href="/quran" class:active={page.url.pathname.startsWith('/quran')}>Qurʾān</a>
      <a class="nav-link" href="/hadiths" class:active={page.url.pathname.startsWith('/hadiths')}>Ḥadīth</a>
      <a class="nav-link" href="/narrators" class:active={page.url.pathname.startsWith('/narrators')}>Narrators</a>
      <a class="nav-link" href="/notes" class:active={page.url.pathname.startsWith('/notes')}>Notes</a>
      {#if $appConfig.advanced_enabled}
        <a class="nav-link nav-link-accent" href="/ask">Chat</a>
      {/if}
    </nav>

    <form class="search-form" onsubmit={handleSearch}>
      <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
        <circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
      </svg>
      <input
        type="text"
        placeholder="Search Qurʾān &amp; Sunnah…"
        bind:value={searchQuery}
        class="search-input"
      />
    </form>

    <div class="actions">
      <QuranSettings />
      <LanguageToggle />
    </div>

    <button class="hamburger" onclick={() => (mobileMenuOpen = !mobileMenuOpen)} aria-label="Menu" aria-expanded={mobileMenuOpen}>
      <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        {#if mobileMenuOpen}
          <line x1="6" y1="6" x2="18" y2="18"/><line x1="6" y1="18" x2="18" y2="6"/>
        {:else}
          <line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="18" x2="21" y2="18"/>
        {/if}
      </svg>
    </button>
  </div>

  {#if mobileMenuOpen}
    <div class="mobile-panel">
      <a class="mobile-link" href="/quran" onclick={() => (mobileMenuOpen = false)}>Qurʾān</a>
      <a class="mobile-link" href="/hadiths" onclick={() => (mobileMenuOpen = false)}>Ḥadīth</a>
      <a class="mobile-link" href="/narrators" onclick={() => (mobileMenuOpen = false)}>Narrators</a>
      <a class="mobile-link" href="/notes" onclick={() => (mobileMenuOpen = false)}>Notes</a>
      <a class="mobile-link" href="/search" onclick={() => (mobileMenuOpen = false)}>Search</a>
      <a class="mobile-link" href="/books" onclick={() => (mobileMenuOpen = false)}>Books</a>
      <a class="mobile-link" href="/families" onclick={() => (mobileMenuOpen = false)}>Families</a>
    </div>
  {/if}
</header>

<style>
  .navbar {
    position: sticky;
    top: 0;
    z-index: 40;
    width: 100%;
    height: var(--topbar-height);
    background: var(--topbar-bg);
    color: var(--topbar-fg);
    border-bottom: 1px solid var(--topbar-border);
  }
  @media (min-width: 1024px) {
    .navbar { height: var(--topbar-height-lg); }
  }

  .navbar-inner {
    max-width: var(--page-width);
    margin: 0 auto;
    height: 100%;
    padding: 0 var(--space-4);
    display: flex;
    align-items: center;
    gap: var(--space-5);
  }
  @media (min-width: 1024px) {
    .navbar-inner { padding: 0 var(--space-8); }
  }

  .logo {
    display: inline-flex;
    align-items: baseline;
    gap: var(--space-2);
    text-decoration: none;
    color: inherit;
    font-family: var(--font-serif);
    flex-shrink: 0;
  }
  .logo-glyph { color: var(--accent); font-size: 1.1rem; }
  .logo-en {
    font-size: var(--text-lg);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-tight);
  }
  .logo-ar {
    font-family: var(--font-arabic);
    font-size: 1rem;
    color: var(--accent);
    opacity: 0.85;
  }

  .primary {
    display: none;
    align-items: center;
    gap: var(--space-5);
    flex: 1;
  }
  @media (min-width: 1024px) {
    .primary { display: flex; }
  }

  .nav-link {
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-medium);
    color: var(--topbar-fg-muted);
    text-decoration: none;
    padding: var(--space-2) 0;
    border-bottom: 1px solid transparent;
    transition: color var(--transition), border-color var(--transition);
  }
  .nav-link:hover { color: var(--accent); }
  .nav-link.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }
  .nav-link-accent { color: var(--accent); }

  .search-form {
    display: none;
    align-items: center;
    gap: var(--space-2);
    flex: 1 1 320px;
    max-width: 360px;
    padding: var(--space-2) var(--space-4);
    background: rgba(245, 236, 215, 0.06);
    border: 1px solid var(--topbar-border);
    border-radius: var(--radius-pill);
    transition: border-color var(--transition);
  }
  .search-form:focus-within { border-color: var(--accent); }
  @media (min-width: 768px) {
    .search-form { display: flex; }
  }

  .search-icon {
    width: 16px;
    height: 16px;
    color: currentColor;
    opacity: 0.65;
    flex-shrink: 0;
  }
  .search-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: inherit;
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    min-width: 0;
  }
  .search-input::placeholder { color: var(--topbar-fg-muted); }

  .actions {
    display: none;
    align-items: center;
    gap: var(--space-2);
    flex-shrink: 0;
  }
  @media (min-width: 768px) {
    .actions { display: flex; }
  }

  .hamburger {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border-radius: var(--radius);
    background: none;
    border: none;
    color: inherit;
    cursor: pointer;
    margin-left: auto;
  }
  @media (min-width: 1024px) {
    .hamburger { display: none; }
  }

  .mobile-panel {
    background: var(--topbar-bg);
    border-top: 1px solid var(--topbar-border);
    display: flex;
    flex-direction: column;
    padding: var(--space-2) var(--space-4);
  }
  .mobile-link {
    padding: var(--space-3) 0;
    border-bottom: 1px solid var(--topbar-border);
    color: inherit;
    text-decoration: none;
    font-family: var(--font-sans);
    font-size: var(--text-body);
  }
</style>
