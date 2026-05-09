<script lang="ts">
  import type { Book } from '$lib/types';

  let { book, currentPage, totalPages, onToggleSidebar }: {
    book: Book;
    currentPage: number;
    totalPages: number;
    onToggleSidebar: () => void;
  } = $props();
</script>

<header class="reader-header">
  <div class="crumb">
    <span class="crumb-label">Book</span>
    <span class="crumb-divider">/</span>
    <span class="crumb-name">{book.name_en ?? book.name_ar}</span>
  </div>
  <div class="header-controls">
    <span class="page-indicator">{currentPage + 1} / {totalPages}</span>
    <button class="toggle-btn" onclick={onToggleSidebar} aria-label="Toggle panel">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="18" x2="21" y2="18"/>
      </svg>
    </button>
  </div>
</header>

<style>
  .reader-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-2) var(--space-5);
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    flex-shrink: 0;
    gap: var(--space-4);
    min-height: 44px;
  }
  .crumb {
    min-width: 0;
    flex: 1;
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    overflow: hidden;
  }
  .crumb-label {
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .crumb-divider { color: var(--text-muted); font-size: var(--text-meta); }
  .crumb-name {
    font-family: var(--font-serif);
    font-size: var(--text-meta);
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .header-controls { display: flex; align-items: center; gap: var(--space-3); flex-shrink: 0; }
  .page-indicator {
    font-family: var(--font-mono);
    font-size: var(--text-meta);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .toggle-btn {
    width: 32px;
    height: 32px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition);
  }
  .toggle-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-muted);
  }
</style>
