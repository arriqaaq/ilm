<script lang="ts">
  let open = $state(false);
  let timeout: ReturnType<typeof setTimeout> | undefined;

  function show() {
    if (timeout) clearTimeout(timeout);
    open = true;
  }
  function hide() {
    timeout = setTimeout(() => (open = false), 120);
  }
</script>

<div class="browse-wrap" onmouseenter={show} onmouseleave={hide} role="presentation">
  <button class="browse-trigger" type="button" aria-haspopup="true" aria-expanded={open}>
    Browse
    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
      <path d="M3 4.5l3 3 3-3"/>
    </svg>
  </button>

  {#if open}
    <div class="menu" role="menu" tabindex="-1" onmouseenter={show} onmouseleave={hide}>
      <div class="col">
        <div class="col-label">Reading</div>
        <a class="item" href="/quran">Qurʾān</a>
        <a class="item" href="/tafsir">Tafsir</a>
        <a class="item" href="/hadiths">Hadiths</a>
        <a class="item" href="/books">Books</a>
      </div>
      <div class="col">
        <div class="col-label">Reference</div>
        <a class="item" href="/narrators">Narrators</a>
        <a class="item" href="/families">Families</a>
        <a class="item" href="/search/isnad">Isnad Search</a>
      </div>
      <div class="col">
        <div class="col-label">Tools</div>
        <a class="item" href="/search">Search</a>
        <a class="item" href="/diff">Matn Diff</a>
        <a class="item" href="/notes">Notes</a>
      </div>
    </div>
  {/if}
</div>

<style>
  .browse-wrap { position: relative; }
  .browse-trigger {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: none;
    border: none;
    color: inherit;
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-medium);
    padding: var(--space-2) 0;
    cursor: pointer;
  }
  .browse-trigger:hover { color: var(--accent); }

  .menu {
    position: absolute;
    top: calc(100% + 8px);
    left: -16px;
    z-index: 50;
    background: var(--bg-surface);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.15);
    padding: var(--space-4);
    display: grid;
    grid-template-columns: repeat(3, minmax(140px, 1fr));
    gap: var(--space-5);
    min-width: 480px;
  }
  .col { display: flex; flex-direction: column; gap: var(--space-1); }
  .col-label {
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    color: var(--accent);
    margin-bottom: var(--space-2);
  }
  .item {
    font-family: var(--font-serif);
    font-size: var(--text-body);
    color: var(--text-primary);
    text-decoration: none;
    padding: var(--space-1) 0;
    transition: color var(--transition);
  }
  .item:hover { color: var(--accent); }
</style>
