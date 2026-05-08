<script lang="ts">
  import { iconPaths, type IconName } from './iconPaths';

  type Size = 'xs' | 'sm' | 'md' | 'lg' | 'xl';

  let { name, size = 'md', strokeWidth = 2, label, class: className = '' }: {
    name: IconName;
    size?: Size;
    strokeWidth?: number;
    label?: string;
    class?: string;
  } = $props();

  const sizeMap: Record<Size, string> = {
    xs: 'var(--icon-xs)',
    sm: 'var(--icon-sm)',
    md: 'var(--icon-md)',
    lg: 'var(--icon-lg)',
    xl: 'var(--icon-xl)',
  };

  let dim = $derived(sizeMap[size]);
</script>

<svg
  class="icon {className}"
  style:width={dim}
  style:height={dim}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={strokeWidth}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden={!label}
  aria-label={label}
  role={label ? 'img' : undefined}
>
  {@html iconPaths[name]}
</svg>

<style>
  .icon {
    display: inline-block;
    flex-shrink: 0;
    vertical-align: middle;
  }
</style>
