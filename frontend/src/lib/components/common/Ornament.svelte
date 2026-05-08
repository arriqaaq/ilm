<script lang="ts">
  let { variant = 'divider', size = 16, color = 'var(--accent)' }: {
    variant?: 'star' | 'divider' | 'corner';
    size?: number;
    color?: string;
  } = $props();
</script>

{#if variant === 'star'}
  <svg class="ornament" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} stroke-width="1.2" stroke-linejoin="round" aria-hidden="true">
    <path d="M12 2 L14.5 9.5 L22 12 L14.5 14.5 L12 22 L9.5 14.5 L2 12 L9.5 9.5 Z" />
    <circle cx="12" cy="12" r="1.4" fill={color} stroke="none" />
  </svg>
{:else if variant === 'corner'}
  <svg class="ornament" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} stroke-width="1.2" stroke-linecap="round" aria-hidden="true">
    <path d="M2 2 L10 2 M2 2 L2 10" />
    <path d="M2 2 Q6 6 10 2" opacity="0.55" />
  </svg>
{:else}
  <div class="divider" role="separator" aria-hidden="true">
    <span class="line" style:background="linear-gradient(to right, transparent, {color}, transparent)"></span>
    <svg class="medallion" width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} stroke-width="1.2" stroke-linejoin="round">
      <path d="M12 2 L14.5 9.5 L22 12 L14.5 14.5 L12 22 L9.5 14.5 L2 12 L9.5 9.5 Z" />
      <circle cx="12" cy="12" r="1.4" fill={color} stroke="none" />
    </svg>
    <span class="line" style:background="linear-gradient(to left, transparent, {color}, transparent)"></span>
  </div>
{/if}

<style>
  .ornament {
    display: inline-block;
    vertical-align: middle;
    flex-shrink: 0;
  }
  .divider {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    opacity: 0.7;
  }
  .line {
    flex: 1;
    height: 1px;
  }
  .medallion {
    flex-shrink: 0;
  }
</style>
