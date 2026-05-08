<script lang="ts">
  import { preferences, stepSize, DEFAULTS, type Theme, type QuranFontMode } from '$lib/stores/preferences';

  let open = $state(false);

  const themes: { key: Theme; label: string; color: string }[] = [
    { key: 'light', label: 'Light', color: '#fdfaf3' },
    { key: 'dark', label: 'Night', color: '#15110b' },
    { key: 'brown', label: 'Sepia', color: '#f5ecd7' },
  ];

  function setTheme(t: Theme) {
    preferences.update(p => ({ ...p, theme: t }));
  }
  function incArabic() {
    preferences.update(p => ({ ...p, arabicFontSize: stepSize(p.arabicFontSize, 1) }));
  }
  function decArabic() {
    preferences.update(p => ({ ...p, arabicFontSize: stepSize(p.arabicFontSize, -1) }));
  }
  function incEnglish() {
    preferences.update(p => ({ ...p, englishFontSize: stepSize(p.englishFontSize, 1) }));
  }
  function decEnglish() {
    preferences.update(p => ({ ...p, englishFontSize: stepSize(p.englishFontSize, -1) }));
  }
  function reset() {
    preferences.set({ ...DEFAULTS });
  }

  function handleClickOutside(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (!target.closest('.settings-wrapper')) {
      open = false;
    }
  }
</script>

<svelte:window onclick={handleClickOutside} />

<div class="settings-wrapper">
  <button class="settings-btn" onclick={() => open = !open} title="Settings">
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <circle cx="12" cy="12" r="3"/>
      <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
    </svg>
  </button>

  {#if open}
    <div class="dropdown">
      <div class="dropdown-title">Settings</div>

      <div class="control-row">
        <span class="control-label">Theme</span>
        <div class="theme-row">
          {#each themes as t}
            <button
              class="theme-dot"
              class:active={$preferences.theme === t.key}
              style:background={t.color}
              onclick={() => setTheme(t.key)}
              title={t.label}
            ></button>
          {/each}
        </div>
      </div>

      <div class="section-label">Quran Script</div>

      <div class="control-row">
        <span class="control-label">Font</span>
        <div class="font-row">
          {#each [
            { key: 'uthmani', label: 'Uthmani' },
            { key: 'madani', label: 'Madani' },
            { key: 'tajweed', label: 'Tajweed' },
          ] as f}
            <button
              class="font-pill"
              class:active={$preferences.quranFont === f.key}
              onclick={() => preferences.update(p => ({ ...p, quranFont: f.key as QuranFontMode }))}
            >{f.label}</button>
          {/each}
        </div>
      </div>

      <div class="section-label">Font Size</div>

      <div class="control-row">
        <span class="control-label">Arabic</span>
        <div class="stepper">
          <button class="step-btn" onclick={decArabic}>-</button>
          <span class="step-value">{$preferences.arabicFontSize.toFixed(1)}</span>
          <button class="step-btn" onclick={incArabic}>+</button>
        </div>
      </div>

      <div class="control-row">
        <span class="control-label">English</span>
        <div class="stepper">
          <button class="step-btn" onclick={decEnglish}>-</button>
          <span class="step-value">{$preferences.englishFontSize.toFixed(1)}</span>
          <button class="step-btn" onclick={incEnglish}>+</button>
        </div>
      </div>

      <button class="reset-btn" onclick={reset}>Reset</button>
    </div>
  {/if}
</div>

<style>
  .settings-wrapper {
    position: relative;
  }
  .settings-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: var(--btn-height-md);
    height: var(--btn-height-md);
    border-radius: var(--radius-full);
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all var(--transition);
  }
  .settings-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .dropdown {
    position: absolute;
    top: 42px;
    right: 0;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: var(--space-3) var(--space-4);
    min-width: 220px;
    box-shadow: var(--card-shadow-hover);
    z-index: 100;
  }
  .dropdown-title {
    font-size: var(--text-2xs);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    margin-bottom: var(--space-3);
  }
  .section-label {
    font-size: var(--text-2xs);
    font-weight: var(--font-weight-semibold);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--text-muted);
    margin: var(--space-3) 0 var(--space-2);
    padding-top: var(--space-2);
    border-top: 1px solid var(--border-subtle);
  }
  .control-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: var(--space-3);
  }
  .control-label {
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }
  .theme-row {
    display: flex;
    gap: var(--space-2);
  }
  .theme-dot {
    width: var(--icon-lg);
    height: var(--icon-lg);
    border-radius: var(--radius-full);
    border: 2px solid var(--border);
    cursor: pointer;
    transition: all var(--transition);
    padding: 0;
  }
  .theme-dot:hover {
    transform: scale(1.1);
  }
  .theme-dot.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-muted);
  }
  .font-row {
    display: flex;
    gap: var(--space-1);
  }
  .font-pill {
    padding: 3px 10px;
    border-radius: var(--radius-pill);
    border: 1px solid var(--border);
    background: var(--bg-hover);
    color: var(--text-secondary);
    font-size: var(--text-2xs);
    font-weight: var(--font-weight-medium);
    cursor: pointer;
    transition: all var(--transition);
  }
  .font-pill:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .font-pill.active {
    background: var(--accent-muted);
    border-color: var(--accent);
    color: var(--accent);
    font-weight: var(--font-weight-semibold);
  }
  .stepper {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .step-btn {
    width: var(--btn-height-sm);
    height: var(--btn-height-sm);
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-hover);
    color: var(--text-primary);
    font-size: var(--text-base);
    font-weight: var(--font-weight-semibold);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition);
  }
  .step-btn:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
  .step-value {
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    color: var(--text-primary);
    min-width: 28px;
    text-align: center;
  }
  .reset-btn {
    width: 100%;
    margin-top: var(--space-1);
    padding: var(--space-1) 0;
    font-size: var(--text-2xs);
    color: var(--text-muted);
    background: none;
    border: none;
    cursor: pointer;
    transition: color var(--transition);
  }
  .reset-btn:hover {
    color: var(--accent);
  }
</style>
