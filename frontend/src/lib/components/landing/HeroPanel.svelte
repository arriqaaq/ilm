<script lang="ts">
  import { goto } from '$app/navigation';
  import { appConfig } from '$lib/stores/config';

  let searchQuery = $state('');
  const suggestions = [
    'What does the Qurʾān say about patience?',
    'Hadiths about kindness to parents',
    'The chain of narrators of Ṣaḥīḥ Bukhārī #1',
    'Tafsir of Sūrat al-Fātiḥa'
  ];

  function submit(q?: string) {
    const value = (q ?? searchQuery).trim();
    if (!value) return;
    const enc = encodeURIComponent(value);
    if ($appConfig.advanced_enabled) {
      goto(`/explore?q=${enc}&type=semantic`);
    } else {
      goto(`/search?q=${enc}&type=text`);
    }
  }

  function handleSubmit(e: Event) { e.preventDefault(); submit(); }
</script>

<section class="hero">
  <div class="hero-glow"></div>
  <div class="hero-inner">
    <div class="eyebrow">عِلْم · ILM</div>
    <h1 class="headline">Search the Qurʾān &amp; Sunnah</h1>
    <p class="subheadline">
      A complete platform for Islamic scholarship — explore the Qurʾān with tafsīr,
      34K+ hadiths with narrator chains, and interactive isnād graphs.
    </p>

    <form class="search-wrap" onsubmit={handleSubmit}>
      <div class="search-bar">
        <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
          <circle cx="11" cy="11" r="8"/><path d="m21 21-4.35-4.35"/>
        </svg>
        <input type="text" placeholder="Ask a question or search a topic…" bind:value={searchQuery} />
        <button class="submit" type="submit">Search</button>
      </div>
      <div class="chips">
        {#each suggestions as s}
          <button class="chip" type="button" onclick={() => submit(s)}>{s}</button>
        {/each}
      </div>
    </form>

    <div class="ctas">
      <a class="cta cta-secondary" href="/quran">Browse Qurʾān</a>
      <a class="cta cta-secondary" href="/hadiths">Browse Ḥadīth</a>
    </div>
  </div>
</section>

<style>
  .hero {
    position: relative;
    width: 100%;
    background: var(--hero-bg);
    background-image: linear-gradient(180deg, var(--hero-bg-2) 0%, var(--hero-bg) 100%);
    color: var(--hero-text);
    overflow: hidden;
  }
  .hero-glow {
    position: absolute;
    top: -120px; left: 50%;
    transform: translateX(-50%);
    width: 720px; height: 720px;
    border-radius: 50%;
    background: radial-gradient(circle, var(--hero-glow) 0%, transparent 60%);
    pointer-events: none;
  }
  .hero-inner {
    position: relative;
    max-width: 760px;
    margin: 0 auto;
    padding: var(--space-12) var(--space-6) var(--space-10);
    text-align: center;
  }

  .eyebrow {
    font-family: var(--font-arabic);
    font-size: 1rem;
    letter-spacing: var(--tracking-eyebrow);
    color: var(--accent);
    margin-bottom: var(--space-3);
  }
  .headline {
    font-family: var(--font-serif);
    font-size: clamp(2.2rem, 5vw, 3.4rem);
    font-weight: var(--font-weight-semibold);
    line-height: 1.1;
    letter-spacing: var(--tracking-tight);
    color: var(--hero-text);
    margin: 0;
  }
  .subheadline {
    margin: var(--space-4) auto 0;
    max-width: 540px;
    font-family: var(--font-serif);
    font-size: var(--text-body-lg);
    line-height: var(--leading-relaxed);
    color: var(--hero-text-muted);
  }

  .search-wrap { margin-top: var(--space-8); }
  .search-bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    max-width: 640px;
    margin: 0 auto;
    padding: var(--space-2) var(--space-2) var(--space-2) var(--space-4);
    background: rgba(245, 236, 215, 0.06);
    border: 1px solid rgba(245, 236, 215, 0.18);
    border-radius: var(--radius-pill);
    transition: border-color var(--transition);
  }
  .search-bar:focus-within { border-color: var(--accent); }
  .search-icon { width: 18px; height: 18px; color: var(--hero-text-muted); flex-shrink: 0; }
  .search-bar input {
    flex: 1; background: transparent; border: none; outline: none;
    color: var(--hero-text); font-family: var(--font-sans); font-size: var(--text-body);
    padding: var(--space-2) 0;
  }
  .search-bar input::placeholder { color: var(--hero-text-muted); }
  .submit {
    border: none;
    background: var(--accent); color: #15110b;
    font-family: var(--font-sans); font-size: var(--text-xs);
    font-weight: var(--font-weight-semibold); letter-spacing: var(--tracking-wide); text-transform: uppercase;
    padding: 0 var(--space-4); height: 36px;
    border-radius: var(--radius-pill);
    cursor: pointer; transition: background var(--transition);
  }
  .submit:hover { background: var(--accent-hover); }

  .chips {
    display: flex; flex-wrap: wrap; gap: var(--space-2);
    justify-content: center; margin-top: var(--space-4);
  }
  .chip {
    padding: var(--space-1) var(--space-3);
    background: transparent;
    border: 1px solid rgba(245, 236, 215, 0.20);
    border-radius: var(--radius-pill);
    color: var(--hero-text-muted);
    font-family: var(--font-sans); font-size: var(--text-meta);
    cursor: pointer; transition: all var(--transition);
  }
  .chip:hover { background: rgba(245, 236, 215, 0.08); color: var(--hero-text); border-color: var(--accent); }

  .ctas { display: flex; justify-content: center; gap: var(--space-3); margin-top: var(--space-8); }
  .cta {
    padding: var(--space-3) var(--space-5);
    border-radius: var(--radius-pill);
    font-family: var(--font-sans); font-size: var(--text-meta); font-weight: var(--font-weight-semibold);
    text-decoration: none; transition: all var(--transition);
  }
  .cta-secondary { color: var(--hero-text); border: 1px solid rgba(245, 236, 215, 0.25); }
  .cta-secondary:hover { background: rgba(245, 236, 215, 0.08); border-color: var(--accent); color: var(--accent); }
</style>
