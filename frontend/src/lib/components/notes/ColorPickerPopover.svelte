<script lang="ts">
  let { title, colors, onselect, onclose }: {
    title: string;
    colors: { value: string; label: string }[];
    onselect: (color: string) => void;
    onclose: () => void;
  } = $props();

  function handleClickOutside(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.color-picker-popover')) {
      onclose();
    }
  }

  $effect(() => {
    document.addEventListener('click', handleClickOutside, true);
    return () => document.removeEventListener('click', handleClickOutside, true);
  });
</script>

<div class="color-picker-popover">
  <div class="picker-title">{title}</div>
  <div class="picker-grid">
    {#each colors as color}
      <button
        class="color-swatch"
        style="background: {color.value || '#fff'}; {color.value ? '' : 'border: 1.5px dashed var(--border);'}"
        title={color.label}
        onclick={() => onselect(color.value)}
      >
        {#if !color.value}
          <span class="no-color">&times;</span>
        {/if}
      </button>
    {/each}
  </div>
</div>

<style>
  .color-picker-popover {
    position: absolute;
    top: 100%;
    left: 0;
    z-index: 50;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: 0 8px 32px -8px rgba(0,0,0,0.18), 0 0 0 1px rgba(218,221,227,0.2);
    padding: 12px;
    min-width: 200px;
    animation: popIn 0.15s ease-out;
  }
  .picker-title {
    font-size: 0.72rem;
    font-weight: 600;
    color: var(--text-muted);
    margin-bottom: 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
  .picker-grid {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 4px;
  }
  .color-swatch {
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: 1.5px solid rgba(0,0,0,0.08);
    cursor: pointer;
    transition: all 120ms ease;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .color-swatch:hover {
    transform: scale(1.15);
    box-shadow: 0 2px 8px rgba(0,0,0,0.15);
    border-color: var(--border);
  }
  .no-color {
    font-size: 0.8rem;
    color: var(--text-muted);
    line-height: 1;
  }
  @keyframes popIn {
    from { opacity: 0; transform: translateY(-4px) scale(0.96); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }
</style>
