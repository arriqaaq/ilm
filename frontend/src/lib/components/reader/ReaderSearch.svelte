<script lang="ts">
  let { bookId, onNavigate }: { bookId: number; onNavigate: (pageIndex: number) => void } = $props();

  let query = $state('');
  let searched = $state(false);

  function handleSubmit(e: Event) {
    e.preventDefault();
    if (!query.trim()) return;
    searched = true;
  }
</script>

<div class="search-pane">
  <form class="search-form" onsubmit={handleSubmit}>
    <input
      type="text"
      class="search-input"
      placeholder="Search inside this book…"
      bind:value={query}
      aria-label="Search inside book {bookId}"
    />
    <button type="submit" class="search-go">Search</button>
  </form>

  {#if !searched}
    <p class="hint">Find a phrase or word inside this book.</p>
  {:else}
    <div class="empty">
      <p class="hint">In-book search is coming soon.</p>
      <p class="hint-sub">For now, use the global search at the top of the page.</p>
    </div>
  {/if}
</div>

<style>
  .search-pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: var(--space-3) var(--space-4);
    gap: var(--space-3);
  }
  .search-form {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0;
  }
  .search-input {
    grid-column: 1;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-right: none;
    border-radius: var(--radius) 0 0 var(--radius);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    outline: none;
  }
  .search-input:focus { border-color: var(--accent); z-index: 1; }
  .search-go {
    grid-column: 2;
    padding: 8px 16px;
    border: 1px solid var(--border);
    border-radius: 0 var(--radius) var(--radius) 0;
    background: var(--bg-surface);
    color: var(--text-secondary);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-medium);
    cursor: pointer;
    transition: all var(--transition);
  }
  .search-go:hover { background: var(--accent-muted); border-color: var(--accent); color: var(--accent); }

  .hint {
    font-family: var(--font-serif);
    font-style: italic;
    color: var(--text-muted);
    text-align: center;
    margin: var(--space-4) 0 0;
    line-height: 1.6;
  }
  .hint-sub {
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    text-transform: uppercase;
    letter-spacing: var(--tracking-eyebrow);
    color: var(--text-muted);
    text-align: center;
    margin: var(--space-2) 0 0;
    font-style: normal;
  }
  .empty {
    margin-top: var(--space-6);
    display: flex;
    flex-direction: column;
    align-items: center;
  }
</style>
