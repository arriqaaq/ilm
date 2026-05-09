<script lang="ts">
  import { getMustalahStats } from '$lib/api';
  import type { MustalahStatsResponse } from '$lib/types';
  import Badge from '$lib/components/common/Badge.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import PageHeader from '$lib/components/common/PageHeader.svelte';
  import SectionHeading from '$lib/components/common/SectionHeading.svelte';
  import Divider from '$lib/components/common/Divider.svelte';

  let stats: MustalahStatsResponse | null = $state(null);
  let loading = $state(true);

  $effect(() => {
    getMustalahStats()
      .then((s) => { stats = s; })
      .catch((e) => console.error('Failed to load analysis stats:', e))
      .finally(() => { loading = false; });
  });
</script>

<div class="page-shell">
  <PageHeader
    eyebrow="Provenance"
    title="Isnād Analysis"
    subtitle="Structural analysis of transmission chains with scholarly narrator assessments."
  />

  {#if loading}
    <LoadingSpinner />
  {:else if stats}
    <section class="overview">
      <div class="stat-grid stat-grid-3">
        <article class="stat-card">
          <div class="stat-value">{stats.family_count.toLocaleString()}</div>
          <div class="stat-label">Hadith Families</div>
        </article>
        <article class="stat-card">
          <div class="stat-value">{stats.analyzed_count.toLocaleString()}</div>
          <div class="stat-label">Analyzed</div>
        </article>
        <article class="stat-card">
          <div class="stat-value">{stats.evidence_count.toLocaleString()}</div>
          <div class="stat-label">Scholar Assessments</div>
        </article>
      </div>
    </section>

    <Divider variant="hairline" />

    <section class="breadth-section">
      <SectionHeading eyebrow="Distribution" title="Transmission Breadth" level={2} />
      <div class="stat-grid stat-grid-4">
        <article class="stat-card">
          <div class="stat-value">{stats.mutawatir_count.toLocaleString()}</div>
          <div class="stat-label">Mutawātir</div>
        </article>
        <article class="stat-card">
          <div class="stat-value">{stats.mashhur_count.toLocaleString()}</div>
          <div class="stat-label">Mashhūr</div>
        </article>
        <article class="stat-card">
          <div class="stat-value">{stats.aziz_count.toLocaleString()}</div>
          <div class="stat-label">ʿAzīz</div>
        </article>
        <article class="stat-card">
          <div class="stat-value">{stats.gharib_count.toLocaleString()}</div>
          <div class="stat-label">Gharīb</div>
        </article>
      </div>
    </section>

    {#if stats.analyzed_count === 0}
      <Divider variant="hairline" />
      <section class="instructions">
        <SectionHeading eyebrow="Getting Started" title="Run the analysis pipeline" level={2} />
        <ol class="step-list">
          <li>Ingest hadith data: <code>make hadith-ingest</code></li>
          <li>Compute families: <code>hadith analyze --families</code></li>
          <li>Run structural analysis: <code>hadith analyze --mustalah</code></li>
          <li>View results on the <a class="link" href="/families">Families</a> page</li>
        </ol>
      </section>
    {:else}
      <Divider variant="hairline" />
      <section class="next-row">
        <a href="/families" class="next-card">
          <div class="next-meta">
            <span class="next-eyebrow">Continue →</span>
            <h3 class="next-title">Browse Families</h3>
            <p class="next-desc">View hadith families and their chain analysis.</p>
          </div>
        </a>
      </section>
    {/if}

    <Divider variant="ornamental" />

    <section class="methodology">
      <SectionHeading eyebrow="Method" title="Methodology" level={2} />
      <p class="prose">This tool displays <strong>structural analysis</strong> of transmission chains and <strong>scholarly assessments</strong> of narrators from classical <em>rijāl</em> works. No algorithmic grades are computed — only observable facts about the chain and what scholars actually said.</p>
      <p class="prose">Each chain is listed with its narrators and any chronology conflicts. Families are classified by transmission breadth (mutawātir/mashhūr/ʿazīz/gharīb) using the minimum number of narrators at any ṭabaqah. Pivot narrators (high bundle coverage) are surfaced as <em>madār al-isnād</em> candidates.</p>
      <p class="prose">Narrator assessments are sourced from:</p>
      <div class="sources">
        <div class="source-item"><Badge text="Taqrīb" variant="accent" /> Ibn Ḥajar al-ʿAsqalānī, <em>Taqrīb al-Tahdhīb</em></div>
        <div class="source-item"><Badge text="Mīzān" variant="default" /> al-Dhahabī, <em>Mīzān al-Iʿtidāl</em></div>
      </div>
    </section>
  {/if}
</div>

<style>
  .stat-grid {
    display: grid;
    gap: var(--space-3);
  }
  .stat-grid-3 { grid-template-columns: repeat(3, 1fr); }
  .stat-grid-4 { grid-template-columns: repeat(4, 1fr); }

  .stat-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    padding: var(--space-5);
    text-align: center;
  }
  .stat-value {
    font-family: var(--font-serif);
    font-size: 2rem;
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    letter-spacing: var(--tracking-tight);
  }
  .stat-label {
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    color: var(--text-muted);
    margin-top: var(--space-2);
  }

  .breadth-section { margin: var(--space-6) 0; }

  .instructions { padding: var(--space-2) 0; }
  .step-list {
    padding-left: var(--space-5);
    font-family: var(--font-serif);
    font-size: var(--text-body);
    line-height: 1.7;
    color: var(--text-secondary);
  }
  .step-list li { margin-bottom: var(--space-2); }
  code {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    padding: 2px var(--space-2);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: var(--text-meta);
  }
  .link {
    color: var(--accent);
    text-decoration: underline;
    text-underline-offset: 0.2em;
  }

  .next-row { margin: var(--space-4) 0; }
  .next-card {
    display: block;
    padding: var(--space-5);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    text-decoration: none;
    color: inherit;
    transition: all var(--transition);
  }
  .next-card:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
  }
  .next-eyebrow {
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    color: var(--accent);
  }
  .next-title {
    font-family: var(--font-serif);
    font-size: var(--text-lg);
    font-weight: var(--font-weight-semibold);
    margin: var(--space-1) 0 var(--space-1);
  }
  .next-desc {
    margin: 0;
    font-family: var(--font-serif);
    font-size: var(--text-meta);
    color: var(--text-secondary);
  }

  .methodology { margin: var(--space-4) 0 var(--space-8); }
  .prose {
    font-family: var(--font-serif);
    font-size: var(--text-body);
    line-height: 1.7;
    color: var(--text-secondary);
    margin-bottom: var(--space-3);
  }
  .prose strong { color: var(--text-primary); }
  .sources {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-4);
    margin-top: var(--space-4);
  }
  .source-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    font-family: var(--font-serif);
    font-size: var(--text-meta);
    color: var(--text-secondary);
  }

  @media (max-width: 768px) {
    .stat-grid-3, .stat-grid-4 { grid-template-columns: repeat(2, 1fr); }
  }
  @media (max-width: 540px) {
    .stat-grid-3, .stat-grid-4 { grid-template-columns: 1fr; }
  }
</style>
