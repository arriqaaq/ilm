<script lang="ts">
  import type { ApiNarrator } from '$lib/types';
  import { language } from '$lib/stores/language';
  import { bilingualDisplayName } from '$lib/normalize';

  let { narrator }: { narrator: ApiNarrator } = $props();

  let displayName = $derived(bilingualDisplayName(narrator, $language, narrator.id));
</script>

<a href="/narrators/{narrator.id}" class="chip">
  {displayName}
</a>

<style>
  .chip {
    display: inline-flex;
    align-items: center;
    padding: 6px 14px;
    background: var(--accent-muted);
    color: var(--accent);
    border-radius: 20px;
    font-size: 1rem;
    font-weight: 500;
    transition: all var(--transition);
    max-width: 280px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .chip:hover {
    background: var(--accent);
    color: var(--bg-primary);
  }
</style>
