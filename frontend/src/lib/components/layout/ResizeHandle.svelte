<script lang="ts">
  let { ondrag, ondragstart, ondragend }: {
    ondrag: (deltaX: number) => void;
    ondragstart?: () => void;
    ondragend?: () => void;
  } = $props();

  let dragging = $state(false);
  let startX = 0;
  let rafId = 0;
  let pendingDelta = 0;

  function flush() {
    if (pendingDelta !== 0) {
      ondrag(pendingDelta);
      pendingDelta = 0;
    }
    rafId = 0;
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    dragging = true;
    startX = e.clientX;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    if (typeof document !== 'undefined') {
      document.documentElement.classList.add('is-resizing');
    }
    ondragstart?.();
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const delta = e.clientX - startX;
    if (delta === 0) return;
    startX = e.clientX;
    pendingDelta += delta;
    if (rafId === 0) {
      rafId = requestAnimationFrame(flush);
    }
  }

  function onPointerUp(e: PointerEvent) {
    if (!dragging) return;
    dragging = false;
    if (rafId !== 0) {
      cancelAnimationFrame(rafId);
      flush();
    }
    try { (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId); } catch { /* ignore */ }
    if (typeof document !== 'undefined') {
      document.documentElement.classList.remove('is-resizing');
    }
    ondragend?.();
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="resize-handle"
  class:dragging
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
>
  <div class="handle-grip">
    <span class="grip-dot"></span>
    <span class="grip-dot"></span>
    <span class="grip-dot"></span>
  </div>
</div>

<style>
  .resize-handle {
    width: 12px;
    flex-shrink: 0;
    position: relative;
    cursor: col-resize;
    display: flex;
    align-items: center;
    justify-content: center;
    user-select: none;
    touch-action: none;
    z-index: 2;
  }

  .resize-handle::before {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: 50%;
    width: 1px;
    background: var(--border);
    transition: background 150ms ease;
  }

  .resize-handle:hover::before,
  .resize-handle.dragging::before {
    background: var(--accent);
  }

  .handle-grip {
    display: flex;
    flex-direction: column;
    gap: 3px;
    z-index: 1;
    opacity: 0;
    transition: opacity 150ms ease;
  }

  .resize-handle:hover .handle-grip,
  .resize-handle.dragging .handle-grip {
    opacity: 1;
  }

  .grip-dot {
    width: 3px;
    height: 3px;
    border-radius: 50%;
    background: var(--text-muted);
  }

  .resize-handle:hover .grip-dot,
  .resize-handle.dragging .grip-dot {
    background: var(--accent);
  }
</style>
