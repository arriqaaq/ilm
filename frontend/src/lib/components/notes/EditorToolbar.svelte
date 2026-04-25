<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import ColorPickerPopover from './ColorPickerPopover.svelte';

  let { editorEl }: { editorEl?: HTMLDivElement } = $props();

  let isBold = $state(false);
  let isItalic = $state(false);
  let isUnderline = $state(false);
  let isStrikethrough = $state(false);
  let blockType = $state('Normal');
  let showBlockMenu = $state(false);
  let showInsertMenu = $state(false);
  let showTextColorPicker = $state(false);
  let showBgColorPicker = $state(false);

  const BLOCK_TYPES = [
    { label: 'Normal', tag: 'p' },
    { label: 'Heading 2', tag: 'h2' },
    { label: 'Heading 3', tag: 'h3' },
    { label: 'Blockquote', tag: 'blockquote' },
  ];

  const INSERT_ITEMS = [
    { label: 'Horizontal Rule', icon: '―', action: () => execFormat('insertHorizontalRule') },
    { label: 'Bulleted List', icon: '•', action: () => execFormat('insertUnorderedList') },
    { label: 'Numbered List', icon: '1.', action: () => execFormat('insertOrderedList') },
  ];

  const TEXT_COLORS = [
    { value: '', label: 'Default' },
    { value: '#000000', label: 'Black' },
    { value: '#374151', label: 'Dark Gray' },
    { value: '#dc2626', label: 'Red' },
    { value: '#ea580c', label: 'Orange' },
    { value: '#d97706', label: 'Amber' },
    { value: '#059669', label: 'Green' },
    { value: '#0d9488', label: 'Teal' },
    { value: '#2563eb', label: 'Blue' },
    { value: '#7c3aed', label: 'Purple' },
    { value: '#db2777', label: 'Pink' },
    { value: '#6b7280', label: 'Gray' },
    { value: '#92400e', label: 'Brown' },
  ];

  const BG_COLORS = [
    { value: '', label: 'None' },
    { value: '#fef3c7', label: 'Yellow' },
    { value: '#dcfce7', label: 'Green' },
    { value: '#dbeafe', label: 'Blue' },
    { value: '#fce7f3', label: 'Pink' },
    { value: '#ede9fe', label: 'Purple' },
    { value: '#e0f2fe', label: 'Light Blue' },
    { value: '#fef9c3', label: 'Light Yellow' },
    { value: '#f0fdf4', label: 'Mint' },
    { value: '#fdf2f8', label: 'Rose' },
    { value: '#e0e7ff', label: 'Indigo' },
    { value: '#f5f5f4', label: 'Stone' },
  ];

  function execFormat(command: string, value?: string) {
    editorEl?.focus();
    document.execCommand(command, false, value);
    updateState();
  }

  function setBlockType(tag: string) {
    editorEl?.focus();
    document.execCommand('formatBlock', false, tag === 'p' ? 'p' : tag);
    showBlockMenu = false;
    updateState();
  }

  function setTextColor(color: string) {
    editorEl?.focus();
    if (!color) {
      document.execCommand('removeFormat', false);
    } else {
      document.execCommand('foreColor', false, color);
    }
    showTextColorPicker = false;
    updateState();
  }

  function setBgColor(color: string) {
    editorEl?.focus();
    if (!color) {
      document.execCommand('hiliteColor', false, 'transparent');
    } else {
      document.execCommand('hiliteColor', false, color);
    }
    showBgColorPicker = false;
    updateState();
  }

  function updateState() {
    isBold = document.queryCommandState('bold');
    isItalic = document.queryCommandState('italic');
    isUnderline = document.queryCommandState('underline');
    isStrikethrough = document.queryCommandState('strikeThrough');

    const block = document.queryCommandValue('formatBlock');
    if (block === 'h2') blockType = 'Heading 2';
    else if (block === 'h3') blockType = 'Heading 3';
    else if (block === 'blockquote') blockType = 'Blockquote';
    else blockType = 'Normal';
  }

  function closeAllMenus() {
    showBlockMenu = false;
    showInsertMenu = false;
    showTextColorPicker = false;
    showBgColorPicker = false;
  }

  function handleSelectionChange() {
    if (!editorEl) return;
    const sel = window.getSelection();
    if (sel && editorEl.contains(sel.anchorNode)) {
      updateState();
    }
  }

  function handleClickOutside(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.block-menu') && !target.closest('.block-dropdown-btn')) {
      showBlockMenu = false;
    }
    if (!target.closest('.insert-menu') && !target.closest('.insert-dropdown-btn')) {
      showInsertMenu = false;
    }
    if (!target.closest('.color-picker-popover') && !target.closest('.color-btn')) {
      showTextColorPicker = false;
      showBgColorPicker = false;
    }
  }

  onMount(() => {
    document.addEventListener('selectionchange', handleSelectionChange);
    document.addEventListener('click', handleClickOutside);
  });

  onDestroy(() => {
    document.removeEventListener('selectionchange', handleSelectionChange);
    document.removeEventListener('click', handleClickOutside);
  });
</script>

<div class="editor-toolbar-bar">
  <!-- Block type dropdown -->
  <div class="toolbar-group">
    <button
      class="block-dropdown-btn"
      onclick={() => { closeAllMenus(); showBlockMenu = !showBlockMenu; }}
    >
      <span class="block-label">{blockType}</span>
      <span class="dropdown-arrow">&#9662;</span>
    </button>
    {#if showBlockMenu}
      <div class="block-menu">
        {#each BLOCK_TYPES as bt}
          <button
            class="menu-item"
            class:active={blockType === bt.label}
            onclick={() => setBlockType(bt.tag)}
          >
            {bt.label}
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <span class="divider"></span>

  <!-- Inline formatting -->
  <div class="toolbar-group">
    <button class="fmt-btn" class:active={isBold} onclick={() => execFormat('bold')} title="Bold (Ctrl+B)">
      <strong>B</strong>
    </button>
    <button class="fmt-btn italic-btn" class:active={isItalic} onclick={() => execFormat('italic')} title="Italic (Ctrl+I)">
      <em>I</em>
    </button>
    <button class="fmt-btn" class:active={isUnderline} onclick={() => execFormat('underline')} title="Underline (Ctrl+U)">
      <span style="text-decoration: underline">U</span>
    </button>
    <button class="fmt-btn" class:active={isStrikethrough} onclick={() => execFormat('strikeThrough')} title="Strikethrough">
      <span style="text-decoration: line-through">S</span>
    </button>
  </div>

  <span class="divider"></span>

  <!-- Text Color -->
  <div class="toolbar-group">
    <button
      class="fmt-btn color-btn"
      title="Text Color"
      onclick={() => { closeAllMenus(); showTextColorPicker = !showTextColorPicker; }}
    >
      <span class="color-a">A</span>
      <span class="color-bar" style="background: var(--accent)"></span>
    </button>
    {#if showTextColorPicker}
      <ColorPickerPopover
        title="Text Color"
        colors={TEXT_COLORS}
        onselect={setTextColor}
        onclose={() => { showTextColorPicker = false; }}
      />
    {/if}
  </div>

  <!-- Background Color -->
  <div class="toolbar-group">
    <button
      class="fmt-btn color-btn"
      title="Background Color"
      onclick={() => { closeAllMenus(); showBgColorPicker = !showBgColorPicker; }}
    >
      <span class="bg-a">A</span>
      <span class="color-bar bg-bar"></span>
    </button>
    {#if showBgColorPicker}
      <ColorPickerPopover
        title="Background Color"
        colors={BG_COLORS}
        onselect={setBgColor}
        onclose={() => { showBgColorPicker = false; }}
      />
    {/if}
  </div>

  <span class="divider"></span>

  <!-- Lists -->
  <div class="toolbar-group">
    <button class="fmt-btn" onclick={() => execFormat('insertUnorderedList')} title="Bullet List">
      <span class="list-icon">&bull;&#8801;</span>
    </button>
    <button class="fmt-btn" onclick={() => execFormat('insertOrderedList')} title="Numbered List">
      <span class="list-icon">1&#8801;</span>
    </button>
  </div>

  <span class="divider"></span>

  <!-- Insert dropdown -->
  <div class="toolbar-group">
    <button
      class="insert-dropdown-btn"
      onclick={() => { closeAllMenus(); showInsertMenu = !showInsertMenu; }}
    >
      Insert <span class="dropdown-arrow">&#9662;</span>
    </button>
    {#if showInsertMenu}
      <div class="insert-menu">
        {#each INSERT_ITEMS as item}
          <button class="menu-item" onclick={() => { item.action(); showInsertMenu = false; }}>
            <span class="menu-icon">{item.icon}</span>
            {item.label}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .editor-toolbar-bar {
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 10px 20px;
    background: var(--bg-primary);
    border-bottom: 1px solid var(--border-subtle);
    min-height: 42px;
    flex-wrap: wrap;
  }
  .toolbar-group {
    display: flex;
    align-items: center;
    gap: 2px;
    position: relative;
  }
  .divider {
    width: 1px;
    height: 18px;
    background: var(--border);
    margin: 0 6px;
    flex-shrink: 0;
    opacity: 0.5;
  }

  .fmt-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 28px;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.85rem;
    font-family: var(--font-sans);
    transition: all var(--transition);
  }
  .fmt-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .fmt-btn.active {
    background: color-mix(in srgb, var(--accent) 12%, transparent);
    color: var(--accent);
  }
  .italic-btn {
    font-style: italic;
  }
  .list-icon {
    font-size: 0.75rem;
    font-family: var(--font-sans);
    letter-spacing: -1px;
  }

  /* Color buttons */
  .color-btn {
    flex-direction: column;
    gap: 1px;
    width: 30px;
    height: 30px;
  }
  .color-a {
    font-size: 0.85rem;
    font-weight: 700;
    font-family: var(--font-serif);
    line-height: 1;
  }
  .bg-a {
    font-size: 0.85rem;
    font-weight: 700;
    font-family: var(--font-serif);
    line-height: 1;
    background: linear-gradient(135deg, #fef3c7 0%, #dcfce7 50%, #dbeafe 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }
  .color-bar {
    width: 14px;
    height: 3px;
    border-radius: 1px;
  }
  .bg-bar {
    background: linear-gradient(90deg, #fef3c7, #dcfce7, #dbeafe);
  }

  .block-dropdown-btn,
  .insert-dropdown-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 12px;
    height: 28px;
    border: none;
    border-radius: 6px;
    background: none;
    color: var(--text-muted);
    cursor: pointer;
    font-size: 0.78rem;
    font-family: var(--font-sans);
    transition: all var(--transition);
    white-space: nowrap;
  }
  .block-dropdown-btn:hover,
  .insert-dropdown-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .block-label {
    font-weight: 500;
  }
  .dropdown-arrow {
    font-size: 0.6rem;
    color: var(--text-muted);
  }

  .block-menu,
  .insert-menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius-lg);
    box-shadow: 0 4px 16px rgba(0,0,0,0.1), 0 0 0 1px rgba(218,221,227,0.2);
    z-index: 100;
    min-width: 160px;
    overflow: hidden;
  }
  .menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    text-align: left;
    padding: 9px 16px;
    border: none;
    background: none;
    color: var(--text-primary);
    font-size: 0.8rem;
    font-family: var(--font-sans);
    cursor: pointer;
    transition: background var(--transition);
    border-radius: 6px;
    margin: 0 4px;
    width: calc(100% - 8px);
  }
  .menu-item:hover {
    background: var(--bg-hover);
  }
  .menu-item.active {
    color: var(--accent);
    font-weight: 600;
  }
  .menu-icon {
    width: 18px;
    text-align: center;
    font-size: 0.85rem;
    color: var(--text-muted);
  }
</style>
