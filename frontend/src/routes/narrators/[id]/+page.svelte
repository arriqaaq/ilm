<script lang="ts">
  import { page } from '$app/state';
  import { getNarrator, getNarratorGraph, updateNarrator, getNarratorBooks, getBookPages } from '$lib/api';
  import type { NarratorDetailResponse, GraphData, NarratorBookRef, BookPage } from '$lib/types';
  import NarratorChip from '$lib/components/narrator/NarratorChip.svelte';
  import HadithCard from '$lib/components/hadith/HadithCard.svelte';
  import GraphView from '$lib/components/graph/GraphView.svelte';
  import LoadingSpinner from '$lib/components/common/LoadingSpinner.svelte';
  import ReaderPage from '$lib/components/reader/ReaderPage.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import PageHero from '$lib/components/layout/PageHero.svelte';
  import TabStrip from '$lib/components/layout/TabStrip.svelte';

  let data: NarratorDetailResponse | null = $state(null);
  let graphData: GraphData | null = $state(null);
  let narratorBooks: NarratorBookRef[] = $state([]);
  let selectedBookRef: NarratorBookRef | null = $state(null);
  let bioPage: BookPage | null = $state(null);
  let bioPageLoading = $state(false);
  let bioCurrentIndex = $state(0);
  let loading = $state(true);
  let activeTab: 'network' | 'hadiths' | 'connections' | 'readbio' | 'details' = $state('network');
  let saving = $state(false);
  let saveMsg = $state('');

  // Editable fields
  let editGender = $state('');
  let editGeneration = $state('');
  let editBio = $state('');
  let editKunya = $state('');
  let editBirthYear = $state('');
  let editBirthCalendar = $state('hijri');
  let editDeathYear = $state('');
  let editDeathCalendar = $state('hijri');
  let editLocations = $state('');
  let editTags = $state('');

  let id = $derived(page.params.id);

  async function loadBioPage(bookRef: NarratorBookRef, pageIndex: number) {
    bioPageLoading = true;
    bioPage = null;
    bioCurrentIndex = pageIndex;
    try {
      const res = await getBookPages(bookRef.book_id, pageIndex, 1);
      if (res.pages.length > 0) bioPage = res.pages[0];
    } catch (e) {
      console.error('Failed to load bio page:', e);
    } finally {
      bioPageLoading = false;
    }
  }

  function selectBook(ref: NarratorBookRef) {
    selectedBookRef = ref;
    loadBioPage(ref, ref.page_index);
  }

  function populateForm() {
    if (!data) return;
    const n = data.narrator;
    editGender = n.gender ?? '';
    editGeneration = n.generation ?? '';
    editBio = n.bio ?? '';
    editKunya = n.kunya ?? '';
    editBirthYear = n.birth_year?.toString() ?? '';
    editBirthCalendar = n.birth_calendar ?? 'hijri';
    editDeathYear = n.death_year?.toString() ?? '';
    editDeathCalendar = n.death_calendar ?? 'hijri';
    editLocations = n.locations?.join(', ') ?? '';
    editTags = n.tags?.join(', ') ?? '';
  }

  $effect(() => {
    if (!id) return;
    loading = true;
    activeTab = 'network';
    Promise.all([getNarrator(id), getNarratorGraph(id)])
      .then(([d, g]) => {
        const seen = new Set<string>();
        d.hadiths = d.hadiths.filter(h => {
          if (seen.has(h.id)) return false;
          seen.add(h.id);
          return true;
        });
        data = d;
        graphData = g;
        // Fetch book references for this narrator
        getNarratorBooks(id)
          .then(books => {
            narratorBooks = books;
            if (books.length > 0) selectBook(books[0]);
          })
          .catch(() => {});
        populateForm();
      })
      .catch((e) => console.error('Failed to load narrator:', e))
      .finally(() => { loading = false; });
  });


  async function handleSave() {
    if (!data) return;
    saving = true;
    saveMsg = '';
    const payload: Record<string, unknown> = {};

    if (editGender) payload.gender = editGender;
    if (editGeneration) payload.generation = editGeneration;
    if (editBio) payload.bio = editBio;
    if (editKunya) payload.kunya = editKunya;
    if (editBirthYear) payload.birth_year = parseInt(editBirthYear);
    if (editBirthCalendar) payload.birth_calendar = editBirthCalendar;
    if (editDeathYear) payload.death_year = parseInt(editDeathYear);
    if (editDeathCalendar) payload.death_calendar = editDeathCalendar;
    if (editLocations.trim()) payload.locations = editLocations.split(',').map(s => s.trim()).filter(Boolean);
    if (editTags.trim()) payload.tags = editTags.split(',').map(s => s.trim()).filter(Boolean);

    try {
      await updateNarrator(data.narrator.id, payload);
      saveMsg = 'Saved';
      // Refresh data
      const d = await getNarrator(id!);
      const seen = new Set<string>();
      d.hadiths = d.hadiths.filter(h => { if (seen.has(h.id)) return false; seen.add(h.id); return true; });
      data = d;
      populateForm();
    } catch (e) {
      saveMsg = 'Error saving';
      console.error(e);
    } finally {
      saving = false;
      setTimeout(() => { saveMsg = ''; }, 3000);
    }
  }
</script>

<div class="narrator-view">
  {#if loading}
    <div class="loading-wrap"><LoadingSpinner /></div>
  {:else if data}
    {@const d = data}
    {@const eyebrowText = d.narrator.generation ? `Narrator · Generation ${d.narrator.generation}` : 'Narrator'}
    {@const otherNames = [
      ...(d.narrator.kunya ? [d.narrator.kunya] : []),
      ...(d.narrator.aliases ?? [])
    ]}

    <PageHero
      nameEn={d.narrator.name_en ?? undefined}
      nameAr={d.narrator.name_ar ?? undefined}
      arabicFont="prose"
    >
      {#snippet eyebrow()}<Eyebrow>{eyebrowText}</Eyebrow>{/snippet}
      {#snippet meta()}
        {#if d.narrator.death_year}
          <span class="hero-meta-item"><span class="hero-meta-label">d.</span> {d.narrator.death_year} {d.narrator.death_calendar === 'gregorian' ? 'CE' : 'AH'}</span>
        {:else if d.narrator.birth_year}
          <span class="hero-meta-item"><span class="hero-meta-label">b.</span> {d.narrator.birth_year} {d.narrator.birth_calendar === 'gregorian' ? 'CE' : 'AH'}</span>
        {/if}
        {#if d.hadiths.length > 0}<span class="hero-dot">·</span><span class="hero-meta-item">{d.hadiths.length.toLocaleString()} {d.hadiths.length === 1 ? 'hadith' : 'hadiths'}</span>{/if}
        {#if d.teachers.length > 0}<span class="hero-dot">·</span><span class="hero-meta-item">{d.teachers.length} {d.teachers.length === 1 ? 'teacher' : 'teachers'}</span>{/if}
        {#if d.students.length > 0}<span class="hero-dot">·</span><span class="hero-meta-item">{d.students.length} {d.students.length === 1 ? 'student' : 'students'}</span>{/if}
        {#if d.narrator.locations && d.narrator.locations.length > 0}<span class="hero-dot">·</span><span class="hero-meta-item">{d.narrator.locations.join(', ')}</span>{/if}
      {/snippet}
    </PageHero>

    {#if otherNames.length > 0}
      <p class="hero-also-known">Also known as <span class="known-names arabic-prose" dir="rtl">{otherNames.join(' · ')}</span></p>
    {/if}
    {#if d.narrator.bio}
      <p class="hero-bio">{d.narrator.bio}</p>
    {/if}

    <TabStrip
      ariaLabel="Narrator sections"
      bind:active={activeTab}
      tabs={[
        { id: 'network', label: 'Network' },
        { id: 'hadiths', label: 'Hadiths', count: d.hadiths.length },
        { id: 'connections', label: 'Connections' },
        ...(narratorBooks.length > 0 ? [{ id: 'readbio' as const, label: 'Read Bio' }] : []),
        { id: 'details', label: 'Details' },
      ]}
    />

    <div class="tab-content" class:tab-content-network={activeTab === 'network'} style="margin-top: var(--space-6)">
      {#if activeTab === 'network'}
        <GraphView data={graphData} />
      {:else if activeTab === 'hadiths'}
        <div class="hadith-list">
          {#each data.hadiths as hadith (hadith.id)}
            <HadithCard {hadith} />
          {/each}
          {#if data.hadiths.length === 0}
            <div class="empty">No hadiths linked to this narrator.</div>
          {/if}
        </div>
      {:else if activeTab === 'connections'}
        {#if data.teachers.length > 0}
          <div class="connection-group">
            <h3>Teachers (heard from)</h3>
            <div class="chips">{#each data.teachers as teacher}<NarratorChip narrator={teacher} />{/each}</div>
          </div>
        {/if}
        {#if data.students.length > 0}
          <div class="connection-group">
            <h3>Students (narrated to)</h3>
            <div class="chips">{#each data.students as student}<NarratorChip narrator={student} />{/each}</div>
          </div>
        {/if}
        {#if data.teachers.length === 0 && data.students.length === 0}
          <div class="empty">No connections found.</div>
        {/if}
      {:else if activeTab === 'readbio'}
        <div class="readbio-tab">
          {#if narratorBooks.length > 1}
            <div class="bio-book-selector">
              <label class="bio-book-label" for="bio-book-select">Book</label>
              <select id="bio-book-select" class="bio-book-select" onchange={(e) => {
                const idx = parseInt((e.target as HTMLSelectElement).value);
                const ref = narratorBooks[idx];
                if (ref) selectBook(ref);
              }}>
                {#each narratorBooks as book, i}
                  <option value={i} selected={book === selectedBookRef}>{book.book_name}</option>
                {/each}
              </select>
            </div>
          {:else if narratorBooks.length === 1}
            <div class="bio-book-header">{narratorBooks[0].book_name}</div>
          {/if}

          <div class="bio-reader">
            {#if bioPageLoading}
              <div class="bio-loading">Loading...</div>
            {:else if bioPage}
              <ReaderPage page={bioPage} />
            {:else}
              <div class="bio-loading">No page available</div>
            {/if}
          </div>

          <div class="bio-nav">
            <button class="bio-nav-btn" onclick={() => { if (selectedBookRef) loadBioPage(selectedBookRef, bioCurrentIndex + 1); }} disabled={bioPageLoading}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="15 18 9 12 15 6"/></svg>
              Next
            </button>
            <span class="bio-nav-page">
              {#if bioPage}
                Vol {bioPage.vol} &middot; Page {bioPage.page_num}
              {/if}
            </span>
            <button class="bio-nav-btn" onclick={() => { if (selectedBookRef) loadBioPage(selectedBookRef, bioCurrentIndex - 1); }} disabled={bioPageLoading || bioCurrentIndex <= 0}>
              Prev
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="9 18 15 12 9 6"/></svg>
            </button>
          </div>

          {#if selectedBookRef}
            <div class="bio-full-link">
              <a href="/books/{selectedBookRef.book_id}?page={bioCurrentIndex}">Open full reader &#x2197;</a>
            </div>
          {/if}
        </div>

      {:else if activeTab === 'details'}
        <form class="details-form" onsubmit={(e) => { e.preventDefault(); handleSave(); }}>
          <div class="form-section">
            <h3>Classification</h3>
            <div class="form-row">
              <label>
                <span>Generation</span>
                <input type="text" bind:value={editGeneration} placeholder="e.g., Sahabi, Tabi'i" />
              </label>
              <label>
                <span>Gender</span>
                <input type="text" bind:value={editGender} placeholder="Male / Female" />
              </label>
            </div>
          </div>

          <div class="form-section">
            <h3>Biography</h3>
            <div class="form-row">
              <label>
                <span>Kunya</span>
                <input type="text" bind:value={editKunya} placeholder="e.g., Abu Huraira" />
              </label>
            </div>
            <div class="form-row">
              <label class="half">
                <span>Birth Year</span>
                <input type="number" bind:value={editBirthYear} placeholder="Year" />
              </label>
              <label class="quarter">
                <span>Calendar</span>
                <select bind:value={editBirthCalendar}>
                  <option value="hijri">Hijri</option>
                  <option value="gregorian">Gregorian</option>
                </select>
              </label>
              <label class="half">
                <span>Death Year</span>
                <input type="number" bind:value={editDeathYear} placeholder="Year" />
              </label>
              <label class="quarter">
                <span>Calendar</span>
                <select bind:value={editDeathCalendar}>
                  <option value="hijri">Hijri</option>
                  <option value="gregorian">Gregorian</option>
                </select>
              </label>
            </div>
            <label>
              <span>Locations (comma-separated)</span>
              <input type="text" bind:value={editLocations} placeholder="e.g., Madinah, Makkah, Kufa" />
            </label>
            <label>
              <span>Tags (comma-separated)</span>
              <input type="text" bind:value={editTags} placeholder="e.g., thiqah, hafiz, mujtahid" />
            </label>
            <label>
              <span>Bio</span>
              <textarea bind:value={editBio} rows="4" placeholder="Biographical notes..."></textarea>
            </label>
          </div>

          <div class="form-actions">
            <button type="submit" class="save-btn" disabled={saving}>
              {saving ? 'Saving...' : 'Save Changes'}
            </button>
            {#if saveMsg}
              <span class="save-msg" class:error={saveMsg === 'Error saving'}>{saveMsg}</span>
            {/if}
          </div>
        </form>
      {/if}
    </div>
  {:else}
    <div class="empty">Narrator not found.</div>
  {/if}
</div>


<style>
  .narrator-view {
    max-width: var(--page-width);
    margin: 0 auto;
    padding: var(--space-8) var(--space-6) var(--space-12);
  }
  @media (max-width: 640px) {
    .narrator-view {
      padding: var(--space-5) var(--space-4) var(--space-10);
    }
  }
  .loading-wrap {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-12);
  }

  .hero-also-known {
    margin: 0 0 var(--space-3);
    font-family: var(--font-serif);
    font-size: var(--text-meta);
    color: var(--text-muted);
  }
  .known-names {
    font-size: 1.05rem;
    color: var(--text-secondary);
  }
  .hero-bio {
    margin: 0 0 var(--space-5);
    font-family: var(--font-serif);
    color: var(--text-secondary);
    font-size: var(--text-body);
    line-height: 1.7;
    max-height: 110px;
    overflow: hidden;
    text-overflow: ellipsis;
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
  .connection-group { margin-bottom: var(--space-6); }
  .connection-group h3 {
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    color: var(--accent);
    margin-bottom: var(--space-3);
  }
  .chips { display: flex; flex-wrap: wrap; gap: var(--space-2); }
  .empty {
    text-align: center;
    color: var(--text-muted);
    padding: var(--space-12);
    font-family: var(--font-serif);
    font-style: italic;
  }
  .tab-content-network {
    height: calc(100vh - 240px);
    min-height: 500px;
  }
  /* On tablet and smaller the graph stacks above the sidebar — let the panel
     grow to fit both so neither gets squished. */
  @media (max-width: 1024px) {
    .tab-content-network {
      height: auto;
      min-height: 0;
    }
  }

  /* Read Bio tab */
  .readbio-tab { padding: var(--space-2) 0; }
  .bio-book-selector {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
  }
  .bio-book-label {
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    color: var(--accent);
  }
  .bio-book-select {
    flex: 1;
    max-width: 320px;
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-surface);
    color: var(--text-primary);
    font-size: var(--text-meta);
    outline: none;
  }
  .bio-book-select:focus { border-color: var(--accent); }
  .bio-book-header {
    font-family: var(--font-serif);
    font-size: var(--text-base);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    margin-bottom: var(--space-3);
  }
  .bio-reader {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    padding: var(--space-4) var(--space-5);
    min-height: 200px;
  }
  .bio-loading {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 150px;
    color: var(--text-muted);
    font-size: var(--text-meta);
  }
  .bio-nav {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) 0;
    margin-top: var(--space-2);
  }
  .bio-nav-btn {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-medium);
    color: var(--text-secondary);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
    transition: all var(--transition);
  }
  .bio-nav-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: var(--accent);
    color: var(--accent);
  }
  .bio-nav-btn:disabled { opacity: 0.4; cursor: default; }
  .bio-nav-page {
    font-family: var(--font-mono);
    font-size: var(--text-meta);
    color: var(--text-muted);
  }
  .bio-full-link { text-align: center; margin-top: var(--space-2); }
  .bio-full-link a {
    font-size: var(--text-meta);
    color: var(--text-muted);
    text-decoration: none;
  }
  .bio-full-link a:hover {
    color: var(--accent);
    text-decoration: underline;
  }

  /* Details form */
  .details-form {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
  }
  .form-section {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    padding: var(--space-5);
  }
  .form-section h3 {
    font-family: var(--font-sans);
    font-size: var(--text-eyebrow);
    font-weight: var(--font-weight-semibold);
    letter-spacing: var(--tracking-eyebrow);
    text-transform: uppercase;
    color: var(--accent);
    margin-bottom: var(--space-4);
  }
  .form-row {
    display: flex;
    gap: var(--space-3);
    margin-bottom: var(--space-3);
    flex-wrap: wrap;
  }
  .form-row label { flex: 1; }
  .form-row label.half { flex: 2; }
  .form-row label.quarter { flex: 1; }
  label {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin-bottom: var(--space-3);
  }
  label span {
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-secondary);
  }
  input, select, textarea {
    padding: var(--space-2) var(--space-3);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: var(--text-meta);
    font-family: inherit;
  }
  input:focus, select:focus, textarea:focus {
    border-color: var(--accent);
    outline: none;
  }
  textarea { resize: vertical; }
  .form-actions {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .save-btn {
    padding: var(--space-3) var(--space-5);
    background: var(--accent);
    color: var(--btn-primary-fg);
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-semibold);
    transition: background var(--transition);
  }
  .save-btn:hover:not(:disabled) { background: var(--accent-hover); }
  .save-btn:disabled { opacity: 0.6; cursor: not-allowed; }
  .save-msg {
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--accent);
  }
  .save-msg.error { color: var(--error); }
</style>
