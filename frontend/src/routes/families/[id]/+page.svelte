<script lang="ts">
  import { page } from '$app/state';
  import { getFamily, getMatnDiff, getMustalahFamily } from '$lib/api';
  import type { FamilyDetailResponse, ApiMatnDiff, MustalahFamilyResponse } from '$lib/types';
  import HadithCard from '$lib/components/hadith/HadithCard.svelte';
  import Badge from '$lib/components/common/Badge.svelte';
  import GlossaryTooltip from '$lib/components/hadith/GlossaryTooltip.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import DiffViewer from '$lib/components/hadith/DiffViewer.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import PageHero from '$lib/components/layout/PageHero.svelte';
  import TabStrip from '$lib/components/layout/TabStrip.svelte';

  let data: FamilyDetailResponse | null = $state(null);
  let mustalah: MustalahFamilyResponse | null = $state(null);
  let loading = $state(true);
  let activeTab: 'variants' | 'analysis' | 'diff' = $state('variants');
  let diffResult: ApiMatnDiff | null = $state(null);
  let diffA = $state('');
  let diffB = $state('');
  let diffLoading = $state(false);

  let expandedChains: Set<number> = $state(new Set());

  let id = $derived(page.params.id);

  $effect(() => {
    if (!id) return;
    loading = true;
    Promise.all([
      getFamily(id),
      getMustalahFamily(id).catch(() => null),
    ])
      .then(([d, m]) => { data = d; mustalah = m; })
      .catch((e) => console.error('Failed to load family:', e))
      .finally(() => { loading = false; });
  });

  function breadthLabel(b: string | null): string {
    if (!b) return '—';
    const labels: Record<string, string> = {
      mutawatir: 'Mutawatir',
      mashhur: 'Mashhur',
      aziz: "'Aziz",
      gharib: 'Gharib',
    };
    return labels[b] ?? b;
  }

  /** Map analysis values to glossary term IDs */
  function glossaryId(value: string | null): string | null {
    if (!value) return null;
    const map: Record<string, string> = {
      mutawatir: 'mutawatir', mashhur: 'mashhur', aziz: 'aziz', gharib: 'gharib',
    };
    return map[value] ?? null;
  }

  function toggleChain(idx: number, _narratorIds: string[] | null) {
    if (expandedChains.has(idx)) {
      expandedChains = new Set([...expandedChains].filter(i => i !== idx));
      return;
    }
    expandedChains = new Set([...expandedChains, idx]);
  }

  async function runDiff() {
    if (!diffA || !diffB || diffA === diffB) return;
    diffLoading = true;
    try {
      diffResult = await getMatnDiff(diffA, diffB);
    } catch (e) {
      console.error('Diff failed:', e);
    } finally {
      diffLoading = false;
    }
  }
</script>

<div class="family-view">
  {#if loading}
    <div class="loading-wrap"><LoadingSpinner /></div>
  {:else if data}
    {@const d = data}
    <PageHero nameEn={d.family.family_label ?? 'Unnamed family'}>
      {#snippet eyebrow()}<Eyebrow>Hadith Family · #{d.family.id.slice(0, 8)}</Eyebrow>{/snippet}
      {#snippet meta()}
        <span class="hero-meta-item"><span class="hero-meta-label">Variants</span> {d.hadiths.length}</span>
        {#if mustalah?.analysis?.breadth_class}
          <span class="hero-dot">·</span>
          <span class="hero-meta-item"><span class="hero-meta-label">Breadth</span> {breadthLabel(mustalah.analysis.breadth_class)}</span>
        {/if}
      {/snippet}
    </PageHero>

    <TabStrip
      ariaLabel="Family sections"
      bind:active={activeTab}
      tabs={[
        { id: 'variants', label: 'Variants', count: d.hadiths.length },
        { id: 'analysis', label: 'Analysis' },
        { id: 'diff', label: 'Matn Diff' },
      ]}
    />

    <div class="tab-content" style="margin-top: var(--space-6)">
      {#if activeTab === 'variants'}
        <div class="hadith-list">
          {#each data.hadiths as hadith (hadith.id)}
            <HadithCard {hadith} />
          {/each}
        </div>
      {:else if activeTab === 'analysis'}
        {#if !mustalah?.analysis}
          <div class="empty">
            <p>No structural analysis results yet.</p>
            <p class="hint">Run <code>hadith analyze --mustalah</code> after computing families.</p>
          </div>
        {:else}
          {@const a = mustalah.analysis}
          <!-- Stats Grid -->
          <div class="mustalah-grid">
            <div class="m-card">
              <div class="label">Transmission Breadth</div>
              <div class="value">{#if glossaryId(a.breadth_class)}<GlossaryTooltip termId={glossaryId(a.breadth_class) ?? ''}>{breadthLabel(a.breadth_class)}</GlossaryTooltip>{:else}{breadthLabel(a.breadth_class)}{/if}</div>
              <div class="detail">Min {a.min_breadth} narrator(s) at tabaqah {a.bottleneck_tabaqah ?? '?'}</div>
            </div>
            <div class="m-card">
              <div class="label">Chains</div>
              <div class="value">{a.chain_count}</div>
              <div class="detail">Transmission chain(s)</div>
            </div>
          </div>

          <!-- Defect Flags -->
          {#if a.ilal_flags && a.ilal_flags.length > 0}
            <div class="ilal-section">
              <h3>'Ilal (Defect Flags)</h3>
              <ul>
                {#each a.ilal_flags as flag}
                  <li>{flag}</li>
                {/each}
              </ul>
            </div>
          {/if}

          <!-- Chain Assessments -->
          {#if mustalah.chains.length > 0}
            <div class="section-header">
              <h3>Chain Assessments</h3>
              <p class="section-hint">Click a chain to view narrator scholarly assessments</p>
            </div>
            {#each mustalah.chains as c, idx}
              <div class="chain-card">
                <button class="chain-header" onclick={() => toggleChain(idx, c.narrator_ids)}>
                  <div class="chain-info">
                    <a href="/hadiths/{c.variant_id}" onclick={(e: MouseEvent) => e.stopPropagation()}>{c.variant_id}</a>
                    <span class="chain-meta">
                      <span class="narrator-count">{c.narrator_count} narrators</span>
                      {#if c.has_chronology_conflict}<Badge text="chronology issue" variant="warning" />{/if}
                    </span>
                  </div>
                  <span class="expand-icon">{expandedChains.has(idx) ? '▾' : '▸'}</span>
                </button>
                {#if expandedChains.has(idx) && c.narrator_ids}
                  <div class="chain-narrators">
                    <table>
                      <thead>
                        <tr>
                          <th>#</th>
                          <th>Narrator</th>
                        </tr>
                      </thead>
                      <tbody>
                        {#each c.narrator_ids as nid, nIdx}
                          <tr>
                            <td class="pos">{nIdx + 1}</td>
                            <td><a href="/narrators/{nid}">{nid}</a></td>
                          </tr>
                        {/each}
                      </tbody>
                    </table>
                  </div>
                {/if}
              </div>
            {/each}
          {/if}

          <!-- Pivot Narrators -->
          {#if mustalah.pivots.length > 0}
            <div class="section-header">
              <h3>Madar al-Isnad (Pivot Narrators)</h3>
            </div>
            <div class="analysis-table">
              <table>
                <thead>
                  <tr>
                    <th>Narrator</th>
                    <th>Coverage</th>
                    <th>Fan-out</th>
                    <th>Diversity</th>
                    <th>Bypass</th>
                    <th>Bottleneck</th>
                  </tr>
                </thead>
                <tbody>
                  {#each mustalah.pivots as p}
                    <tr>
                      <td><a href="/narrators/{p.narrator_id}">{p.narrator_id}</a></td>
                      <td class="mono">{(p.bundle_coverage ?? 0).toFixed(2)}</td>
                      <td>{p.fan_out}</td>
                      <td>{p.collector_diversity}</td>
                      <td>{p.bypass_count}</td>
                      <td>{#if p.is_bottleneck}<Badge text="gharabah" variant="warning" />{/if}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        {/if}
      {:else if activeTab === 'diff'}
        <div class="diff-controls">
          <label>
            <span>Hadith A</span>
            <select bind:value={diffA}>
              <option value="">Select...</option>
              {#each data.hadiths as h}
                <option value={h.id}>#{h.hadith_number} — {h.book_name ?? ''}</option>
              {/each}
            </select>
          </label>
          <label>
            <span>Hadith B</span>
            <select bind:value={diffB}>
              <option value="">Select...</option>
              {#each data.hadiths as h}
                <option value={h.id}>#{h.hadith_number} — {h.book_name ?? ''}</option>
              {/each}
            </select>
          </label>
          <button class="diff-btn" onclick={runDiff} disabled={!diffA || !diffB || diffA === diffB || diffLoading}>
            {diffLoading ? 'Computing...' : 'Compare'}
          </button>
        </div>
        {#if diffResult}
          <DiffViewer result={diffResult} />
        {/if}
      {/if}
    </div>
  {:else}
    <div class="empty">Family not found.</div>
  {/if}
</div>

<style>
  .family-view {
    max-width: var(--page-width);
    margin: 0 auto;
    padding: var(--space-8) var(--space-6) var(--space-12);
  }
  .loading-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-12);
  }

  .hero-meta-item {
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-secondary);
  }
  .hero-meta-label {
    color: var(--text-muted);
    font-size: var(--text-eyebrow);
    text-transform: uppercase;
    letter-spacing: var(--tracking-eyebrow);
    font-weight: var(--font-weight-semibold);
    margin-right: 4px;
  }
  .hero-dot { color: var(--text-muted); }

  .hadith-list { display: flex; flex-direction: column; }
  .empty {
    text-align: center;
    color: var(--text-muted);
    padding: var(--space-12);
    font-family: var(--font-serif);
    font-style: italic;
  }
  .hint { font-size: var(--text-meta); }
  code {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    padding: 2px var(--space-2);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: var(--text-meta);
  }

  /* Analysis table */
  .analysis-table { overflow-x: auto; }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-meta);
  }
  th {
    text-align: left;
    padding: var(--space-3);
    border-bottom: 1px solid var(--border-subtle);
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    color: var(--text-muted);
  }
  td {
    padding: var(--space-3);
    border-bottom: 1px solid var(--border-subtle);
  }
  td.mono { font-family: var(--font-mono); }
  td.pos {
    width: 30px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }
  td a { color: var(--accent); }
  td a:hover { text-decoration: underline; }

  /* Diff controls */
  .diff-controls {
    display: flex;
    gap: var(--space-3);
    align-items: flex-end;
    margin-bottom: var(--space-5);
    flex-wrap: wrap;
  }
  .diff-controls label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    flex: 1;
    min-width: 200px;
  }
  .diff-controls span {
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-secondary);
  }
  .diff-controls select {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: var(--text-meta);
  }
  .diff-btn {
    padding: var(--space-2) var(--space-5);
    background: var(--accent);
    color: var(--btn-primary-fg);
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    white-space: nowrap;
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-semibold);
    transition: background var(--transition);
  }
  .diff-btn:hover:not(:disabled) { background: var(--accent-hover); }
  .diff-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  /* Mustalah analysis cards */
  .mustalah-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: var(--space-3);
    margin-bottom: var(--space-5);
  }
  .m-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    padding: var(--space-4);
  }
  .m-card .label {
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    color: var(--text-muted);
    margin-bottom: var(--space-1);
  }
  .m-card .value {
    font-family: var(--font-serif);
    font-size: var(--text-lg);
    font-weight: var(--font-weight-semibold);
  }
  .m-card .detail {
    font-size: var(--text-meta);
    color: var(--text-muted);
    margin-top: var(--space-1);
  }
  .section-header {
    margin-top: var(--space-5);
    margin-bottom: var(--space-3);
  }
  .section-header h3 {
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    color: var(--accent);
  }
  .section-hint {
    font-size: var(--text-meta);
    color: var(--text-muted);
    margin-top: var(--space-1);
  }
  .ilal-section {
    margin: var(--space-4) 0;
    padding: var(--space-4);
    background: color-mix(in srgb, var(--warning) 8%, transparent);
    border: 1px solid var(--warning);
    border-radius: var(--radius);
  }
  .ilal-section h3 {
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    color: var(--warning);
    margin-bottom: var(--space-2);
  }
  .ilal-section ul {
    margin: 0;
    padding-left: var(--space-5);
    font-size: var(--text-meta);
    color: var(--text-secondary);
  }
  .ilal-section li { margin-bottom: var(--space-1); }

  /* Chain cards */
  .chain-card {
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    margin-bottom: var(--space-2);
    overflow: hidden;
  }
  .chain-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: var(--space-3) var(--space-4);
    background: var(--bg-surface);
    cursor: pointer;
    width: 100%;
    border: none;
    color: var(--text-primary);
    text-align: inherit;
    transition: background var(--transition);
  }
  .chain-header:hover { background: var(--bg-hover); }
  .chain-info {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    flex-wrap: wrap;
  }
  .chain-info a {
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-semibold);
  }
  .chain-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .narrator-count {
    font-size: var(--text-meta);
    color: var(--text-muted);
  }
  .expand-icon { font-size: var(--text-meta); color: var(--text-muted); }
  .chain-narrators { border-top: 1px solid var(--border); }
  .chain-narrators table { font-size: 0.85rem; }
  .chain-narrators th { font-size: var(--text-2xs); }

  @media (max-width: 768px) {
  }
</style>
