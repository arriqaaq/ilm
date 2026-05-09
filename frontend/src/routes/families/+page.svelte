<script lang="ts">
  import { getFamilies } from '$lib/api';
  import type { ApiHadithFamily, PaginatedResponse } from '$lib/types';
  import Pagination from '$lib/components/common/Pagination.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import PageHeader from '$lib/components/common/PageHeader.svelte';
  import Badge from '$lib/components/common/Badge.svelte';

  let data: PaginatedResponse<ApiHadithFamily> | null = $state(null);
  let loading = $state(true);
  let currentPage = $state(1);

  async function load() {
    loading = true;
    try {
      data = await getFamilies({ page: currentPage, limit: 20 });
    } catch (e) {
      console.error('Failed to load families:', e);
    } finally {
      loading = false;
    }
  }

  $effect(() => { load(); });

  function onPageChange(page: number) {
    currentPage = page;
    load();
  }
</script>

<div class="page-shell">
  <PageHeader
    eyebrow="Networks"
    title="Hadith Families"
    subtitle="Groups of hadith variants sharing the same report across different chains."
  />

  {#if loading}
    <LoadingSpinner />
  {:else if data && data.data.length > 0}
    <div class="family-list">
      {#each data.data as family (family.id)}
        <a href="/families/{family.id}" class="family-row">
          <div class="family-meta">
            <span class="family-id mono">#{family.id.slice(0, 8)}</span>
            <h3 class="family-label">{family.family_label ?? 'Unnamed family'}</h3>
          </div>
          <Badge text="{family.variant_count ?? 0} variants" variant="accent" />
        </a>
      {/each}
    </div>
    <Pagination page={currentPage} hasMore={data.has_more} {onPageChange} />
  {:else}
    <div class="empty">
      <p class="empty-line">No hadith families computed yet.</p>
      <p class="empty-hint">Run <code>hadith analyze --families</code> to cluster hadiths into families.</p>
    </div>
  {/if}
</div>

<style>
  .family-list {
    display: flex;
    flex-direction: column;
    margin-bottom: var(--space-6);
  }
  .family-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-2);
    border-bottom: 1px solid var(--border-subtle);
    text-decoration: none;
    color: inherit;
    transition: background var(--transition);
  }
  .family-row:hover { background: var(--bg-hover); }
  .family-row:last-child { border-bottom: none; }

  .family-meta {
    display: flex;
    align-items: baseline;
    gap: var(--space-3);
    min-width: 0;
  }
  .family-id {
    font-size: var(--text-eyebrow);
    color: var(--text-muted);
    flex-shrink: 0;
  }
  .family-label {
    font-family: var(--font-serif);
    font-size: var(--text-lg);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    margin: 0;
    line-height: 1.4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .empty {
    text-align: center;
    color: var(--text-muted);
    padding: var(--space-12);
  }
  .empty-line {
    font-family: var(--font-serif);
    font-style: italic;
    font-size: var(--text-body);
    margin: 0 0 var(--space-2);
  }
  .empty-hint {
    font-size: var(--text-meta);
    margin: 0;
  }
  code {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    padding: 2px var(--space-2);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: var(--text-meta);
  }
</style>
