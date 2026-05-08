<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes, HTMLAnchorAttributes } from 'svelte/elements';

  type Variant = 'primary' | 'secondary' | 'ghost' | 'soft';
  type Size = 'sm' | 'md' | 'lg';

  type Props = {
    variant?: Variant;
    size?: Size;
    uppercase?: boolean;
    pill?: boolean;
    iconOnly?: boolean;
    block?: boolean;
    href?: string;
    children?: Snippet;
    class?: string;
  } & Omit<HTMLButtonAttributes & HTMLAnchorAttributes, 'class'>;

  let {
    variant = 'soft',
    size = 'md',
    uppercase = false,
    pill = false,
    iconOnly = false,
    block = false,
    href,
    children,
    class: className = '',
    ...rest
  }: Props = $props();

  let classes = $derived([
    'btn',
    `btn-${variant}`,
    `btn-${size}`,
    uppercase && 'btn-uppercase',
    pill && 'btn-pill',
    iconOnly && 'btn-icon',
    block && 'btn-block',
    className,
  ].filter(Boolean).join(' '));
</script>

{#if href}
  <a {href} class={classes} {...rest}>
    {@render children?.()}
  </a>
{:else}
  <button class={classes} {...rest}>
    {@render children?.()}
  </button>
{/if}
