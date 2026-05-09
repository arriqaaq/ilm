<script lang="ts">
  import type { BookHeading } from '$lib/types';

  let { headings, currentPageIndex, totalPages, onNavigate, onClose }: {
    headings: BookHeading[];
    currentPageIndex: number;
    totalPages: number;
    onNavigate: (pageIndex: number) => void;
    onClose?: () => void;
  } = $props();

  let pageInput = $state('');
  let expandedSections: Set<number> = $state(new Set());

  // Build tree: level 1 headings are parents, level 2+ are children
  interface HeadingNode {
    heading: BookHeading;
    index: number;
    children: HeadingNode[];
  }

  let tree = $derived.by(() => {
    const nodes: HeadingNode[] = [];
    let currentParent: HeadingNode | null = null;

    for (let i = 0; i < headings.length; i++) {
      const h = headings[i];
      if (h.level === 1) {
        currentParent = { heading: h, index: i, children: [] };
        nodes.push(currentParent);
      } else if (currentParent) {
        currentParent.children.push({ heading: h, index: i, children: [] });
      } else {
        nodes.push({ heading: h, index: i, children: [] });
      }
    }
    return nodes;
  });

  // Find which heading is "current" based on scroll position
  let activeHeadingIndex = $derived.by(() => {
    let best = -1;
    for (let i = 0; i < headings.length; i++) {
      if (headings[i].page_index <= currentPageIndex) {
        best = i;
      } else {
        break;
      }
    }
    return best;
  });

  function toggleSection(index: number) {
    const next = new Set(expandedSections);
    if (next.has(index)) {
      next.delete(index);
    } else {
      next.add(index);
    }
    expandedSections = next;
  }

  function handlePageJump() {
    const num = parseInt(pageInput, 10);
    if (num >= 1 && num <= totalPages) {
      onNavigate(num - 1);
    }
    pageInput = '';
  }
</script>

<aside class="reader-sidebar" dir="rtl">
  {#if onClose}
    <button class="close-btn" onclick={onClose} aria-label="Close">&times;</button>
  {/if}

  <form class="page-navigator" onsubmit={(e) => { e.preventDefault(); handlePageJump(); }}>
    <input
      id="reader-page-nav"
      type="number"
      class="nav-input"
      placeholder="Go to page #"
      min="1"
      max={totalPages}
      bind:value={pageInput}
      aria-label="Go to page"
    />
    <button type="submit" class="nav-go">Go</button>
    <span class="nav-total">of {totalPages.toLocaleString()}</span>
  </form>

  <div class="heading-tree">
    {#each tree as node}
      <div class="tree-node">
        {#if node.children.length > 0}
          <button
            class="tree-parent"
            class:active={activeHeadingIndex === node.index || node.children.some(c => c.index === activeHeadingIndex)}
            onclick={() => toggleSection(node.index)}
          >
            <span class="expand-icon" class:expanded={expandedSections.has(node.index)}>&#9656;</span>
            <span class="heading-title">{node.heading.title}</span>
          </button>
          {#if expandedSections.has(node.index)}
            <div class="tree-children">
              {#each node.children as child}
                <button
                  class="tree-child"
                  class:active={child.index === activeHeadingIndex}
                  onclick={() => onNavigate(child.heading.page_index)}
                >
                  {child.heading.title}
                </button>
              {/each}
            </div>
          {/if}
        {:else}
          <button
            class="tree-parent leaf"
            class:active={node.index === activeHeadingIndex}
            onclick={() => onNavigate(node.heading.page_index)}
          >
            <span class="heading-title">{node.heading.title}</span>
          </button>
        {/if}
      </div>
    {/each}
  </div>
</aside>

<style>
  .reader-sidebar {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    background: var(--bg-primary);
    border-left: 1px solid var(--border);
    padding: 16px 0;
    position: relative;
  }
  .close-btn {
    display: none;
    position: absolute;
    top: 8px;
    left: 8px;
    width: 32px;
    height: 32px;
    border: none;
    background: var(--bg-hover);
    border-radius: var(--radius-sm);
    font-size: 1.2rem;
    color: var(--text-muted);
    cursor: pointer;
    align-items: center;
    justify-content: center;
  }
  .close-btn:hover { background: var(--bg-active); }

  .page-navigator {
    display: grid;
    grid-template-columns: 1fr auto;
    grid-template-rows: auto auto;
    column-gap: 0;
    row-gap: 4px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border-subtle);
    direction: ltr;
  }
  .nav-input {
    grid-column: 1;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-right: none;
    border-radius: var(--radius) 0 0 var(--radius);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: var(--text-meta);
    outline: none;
    -moz-appearance: textfield;
    appearance: textfield;
  }
  .nav-input::-webkit-outer-spin-button,
  .nav-input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }
  .nav-input:focus { border-color: var(--accent); z-index: 1; }
  .nav-go {
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
  .nav-go:hover { background: var(--accent-muted); border-color: var(--accent); color: var(--accent); }
  .nav-total {
    grid-column: 1 / -1;
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    text-transform: uppercase;
    letter-spacing: var(--tracking-eyebrow);
    color: var(--text-muted);
    text-align: left;
    margin-top: 4px;
  }

  .heading-tree {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }
  .tree-node {
    margin-bottom: 1px;
  }
  .tree-parent {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    width: 100%;
    padding: 8px 16px;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: var(--text-meta);
    font-weight: 600;
    line-height: 1.6;
    text-align: right;
    cursor: pointer;
    transition: background var(--transition);
    font-family: var(--font-sans);
  }
  .tree-parent:hover { background: var(--bg-hover); }
  .tree-parent.active { color: var(--accent); background: var(--accent-muted); }
  .tree-parent.leaf { font-weight: 500; }
  .expand-icon {
    flex-shrink: 0;
    font-size: 0.7rem;
    transition: transform 0.2s ease;
    margin-top: 4px;
  }
  .expand-icon.expanded { transform: rotate(90deg); }

  .heading-title {
    flex: 1;
  }

  .tree-children {
    padding-right: 20px;
  }
  .tree-child {
    display: block;
    width: 100%;
    padding: 5px 16px;
    border: none;
    background: none;
    color: var(--text-secondary);
    font-size: 0.8rem;
    line-height: 1.6;
    text-align: right;
    cursor: pointer;
    transition: all var(--transition);
    font-family: var(--font-sans);
  }
  .tree-child:hover { background: var(--bg-hover); color: var(--text-primary); }
  .tree-child.active { color: var(--accent); background: var(--accent-muted); font-weight: 600; }

  @media (max-width: 768px) {
    .close-btn { display: flex; }
    .reader-sidebar { padding-top: 44px; }
  }
</style>
