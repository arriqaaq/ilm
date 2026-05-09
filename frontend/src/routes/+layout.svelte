<script lang="ts">
  import '../app.css';
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import AppNavbar from '$lib/components/layout/AppNavbar.svelte';
  import { preferences } from '$lib/stores/preferences';
  import { loadAppConfig } from '$lib/stores/config';

  let { children } = $props();

  let isLanding = $derived(page.url.pathname === '/');

  let navVariant = $derived<'home' | 'default'>(
    isLanding ? 'home' : 'default'
  );

  onMount(() => {
    loadAppConfig();
    const unsub = preferences.subscribe(p => {
      document.documentElement.dataset.theme = p.theme;
    });
    return unsub;
  });
</script>

<svelte:head>
  <title>Ilm</title>
  <meta name="viewport" content="width=device-width, initial-scale=1" />
</svelte:head>

{#if isLanding}
  <AppNavbar variant="home" />
  {@render children()}
{:else}
  <div class="app-shell">
    <AppNavbar variant={navVariant} />
    <main class="content">
      {@render children()}
    </main>
  </div>
{/if}

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow: auto;
  }
</style>
