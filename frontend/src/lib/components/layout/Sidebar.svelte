<script lang="ts">
  import { page } from '$app/state';
  import { appConfig } from '$lib/stores/config';
  import Ornament from '$lib/components/common/Ornament.svelte';

  let { collapsed = false, onToggle }: {
    collapsed?: boolean;
    onToggle?: () => void;
  } = $props();

  interface NavItem {
    path: string;
    label: string;
    icon: string;
    advanced?: boolean;
  }

  interface NavGroup {
    label: string;
    items: NavItem[];
    advanced?: boolean;
  }

  const groups: NavGroup[] = [
    {
      label: 'Browse',
      advanced: true,
      items: [
        { path: '/explore', label: 'Explore', icon: '✦', advanced: true },
        { path: '/ask', label: 'Ask', icon: '◈', advanced: true },
      ],
    },
    {
      label: 'Qurʾān',
      items: [
        { path: '/quran', label: 'Quran', icon: '▣' },
        { path: '/tafsir', label: 'Tafsir', icon: '✧' },
        { path: '/quran/search', label: 'Search', icon: '⌕' },
      ],
    },
    {
      label: 'Notes',
      items: [
        { path: '/notes', label: 'Notes', icon: '✎' },
      ],
    },
    {
      label: 'Ḥadīth',
      items: [
        { path: '/hadiths', label: 'Hadiths', icon: '⛓' },
        { path: '/narrators', label: 'Narrators', icon: '◉' },
        { path: '/search/isnad', label: 'Isnad Search', icon: '⌬' },
        { path: '/books', label: 'Books', icon: '▤' },
        { path: '/families', label: 'Families', icon: '⬡', advanced: true },
        { path: '/diff', label: 'Diff', icon: '⇄' },
        { path: '/search', label: 'Search', icon: '⌕' },
        { path: '/analysis', label: 'Analysis', icon: '△', advanced: true },
      ],
    },
  ];

  let filteredGroups = $derived(
    groups
      .filter(g => !g.advanced || $appConfig.advanced_enabled)
      .map(g => ({
        ...g,
        items: g.items.filter(i => !i.advanced || $appConfig.advanced_enabled),
      }))
      .filter(g => g.items.length > 0)
  );

  function isActive(path: string): boolean {
    const current = page.url.pathname;
    if (path === '/') return current === '/';
    return current === path || current.startsWith(path + '/');
  }
</script>

<nav class="sidebar" class:collapsed>
  <div class="sidebar-header">
    <a href="/" class="logo-link" title="Ilm">
      <span class="logo">❋</span>
      {#if !collapsed}
        <span class="logo-text">Ilm</span>
        <span class="logo-arabic" dir="rtl">عِلْم</span>
      {/if}
    </a>
    <button class="collapse-toggle" onclick={onToggle} title={collapsed ? 'Expand sidebar (Ctrl+B)' : 'Collapse sidebar (Ctrl+B)'}>
      {collapsed ? '»' : '«'}
    </button>
  </div>

  <div class="nav-items">
    {#each filteredGroups as group, i}
      {#if !collapsed && i > 0}
        <div class="group-divider">
          <Ornament variant="divider" size={10} color="var(--border)" />
        </div>
      {/if}
      <div class="nav-group">
        {#if !collapsed}
          <span class="section-label">{group.label}</span>
        {/if}
        {#each group.items as item}
          <a
            href={item.path}
            class="nav-item"
            class:active={isActive(item.path)}
            title={collapsed ? item.label : ''}
          >
            <span class="nav-icon">{item.icon}</span>
            {#if !collapsed}
              <span class="nav-label">{item.label}</span>
            {/if}
          </a>
        {/each}
      </div>
    {/each}
  </div>

  <div class="sidebar-footer">
    {#if !collapsed}
      <div class="footer-stack">
        <span class="footer-text">Islamic Knowledge Platform</span>
        <span class="footer-arabic" dir="rtl">منصة العلوم الإسلامية</span>
      </div>
    {/if}
  </div>
</nav>

<style>
  .sidebar {
    width: var(--sidebar-width);
    height: 100%;
    background: var(--bg-primary);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    transition: width 200ms ease;
    overflow: hidden;
  }

  .sidebar.collapsed {
    width: var(--sidebar-collapsed-width);
  }

  .sidebar-header {
    padding: var(--space-3);
    border-bottom: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: var(--topbar-height);
    white-space: nowrap;
    overflow: hidden;
    flex-shrink: 0;
  }
  .collapsed .sidebar-header {
    justify-content: center;
    padding: var(--space-3) var(--space-1);
  }

  .logo-link {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    text-decoration: none;
    color: inherit;
    transition: opacity var(--transition);
  }
  .logo-link:hover {
    opacity: 0.85;
    color: inherit;
  }

  .logo {
    color: var(--accent);
    font-size: var(--text-md);
    flex-shrink: 0;
  }

  .logo-text {
    font-family: var(--font-serif);
    font-weight: var(--font-weight-semibold);
    font-size: var(--text-lg);
    color: var(--text-primary);
    letter-spacing: var(--tracking-tight);
  }

  .logo-arabic {
    font-family: var(--font-arabic-ui);
    font-size: var(--text-md);
    color: var(--accent);
    margin-left: var(--space-1);
    opacity: 0.85;
    line-height: 1;
  }

  .nav-items {
    flex: 1;
    padding: var(--space-2) 10px;
    display: flex;
    flex-direction: column;
    gap: 0;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .collapsed .nav-items {
    padding: var(--space-2) var(--space-1);
  }

  .nav-group {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .group-divider {
    padding: var(--space-2) 14px;
  }

  .section-label {
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    color: var(--accent);
    padding: var(--space-4) var(--space-3) var(--space-2);
    user-select: none;
    white-space: nowrap;
  }

  .nav-group:first-child .section-label {
    padding-top: var(--space-2);
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 7px var(--space-3);
    border-radius: var(--radius-sm);
    border-left: 3px solid transparent;
    color: var(--text-secondary);
    transition: all var(--transition);
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    text-decoration: none;
    white-space: nowrap;
    overflow: hidden;
  }
  .collapsed .nav-item {
    justify-content: center;
    padding: var(--space-2) 0;
    border-left: none;
    border-radius: var(--radius-sm);
  }

  .nav-item:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .nav-item.active {
    background: var(--accent-muted);
    color: var(--accent);
    border-left-color: var(--accent);
    font-weight: var(--font-weight-semibold);
  }
  .collapsed .nav-item.active {
    border-left-color: transparent;
  }

  .nav-icon {
    width: var(--icon-md);
    text-align: center;
    font-size: var(--text-sm);
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .nav-item.active .nav-icon {
    color: var(--accent);
  }

  .sidebar-footer {
    padding: var(--space-3);
    border-top: 1px solid var(--border-subtle);
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .collapsed .sidebar-footer {
    justify-content: center;
    padding: var(--space-2) var(--space-1);
  }

  .footer-stack {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    overflow: hidden;
  }

  .footer-text {
    font-family: var(--font-serif);
    font-size: var(--text-2xs);
    font-style: italic;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
  }

  .footer-arabic {
    font-family: var(--font-arabic-ui);
    font-size: var(--text-xs);
    color: var(--accent);
    opacity: 0.75;
    line-height: 1.4;
    white-space: nowrap;
    overflow: hidden;
    text-align: right;
  }

  .collapse-toggle {
    width: var(--btn-height-sm);
    height: var(--btn-height-sm);
    display: flex;
    align-items: center;
    justify-content: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-secondary);
    font-size: var(--text-md);
    font-weight: var(--font-weight-bold);
    cursor: pointer;
    transition: all var(--transition);
    flex-shrink: 0;
  }
  .collapse-toggle:hover {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-muted);
  }
  .collapsed .collapse-toggle {
    width: 100%;
    border: none;
    border-radius: 0;
  }
</style>
