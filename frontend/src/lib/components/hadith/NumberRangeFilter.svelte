<script lang="ts">
  import FilterSection from './FilterSection.svelte';

  let {
    min,
    max,
    onChange,
  }: {
    min: number | undefined;
    max: number | undefined;
    onChange: (next: { min?: number; max?: number }) => void;
  } = $props();

  let minInput: number | '' = $state('');
  let maxInput: number | '' = $state('');
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    minInput = min ?? '';
    maxInput = max ?? '';
  });

  function commit() {
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      const parsedMin = typeof minInput === 'number' || minInput !== ''
        ? Number(minInput) : undefined;
      const parsedMax = typeof maxInput === 'number' || maxInput !== ''
        ? Number(maxInput) : undefined;
      onChange({
        min: Number.isFinite(parsedMin) ? (parsedMin as number) : undefined,
        max: Number.isFinite(parsedMax) ? (parsedMax as number) : undefined,
      });
    }, 350);
  }

  const activeCount = $derived(
    (min !== undefined ? 1 : 0) + (max !== undefined ? 1 : 0)
  );
</script>

<FilterSection
  title="Hadith number"
  {activeCount}
  onClear={() => { minInput = ''; maxInput = ''; onChange({}); }}
>
  <div class="range">
    <label class="field">
      <span class="lbl">Min</span>
      <input
        type="number"
        inputmode="numeric"
        min="1"
        placeholder="1"
        bind:value={minInput}
        oninput={commit}
      />
    </label>
    <span class="sep">—</span>
    <label class="field">
      <span class="lbl">Max</span>
      <input
        type="number"
        inputmode="numeric"
        min="1"
        placeholder="∞"
        bind:value={maxInput}
        oninput={commit}
      />
    </label>
  </div>
</FilterSection>

<style>
  .range {
    display: flex;
    align-items: end;
    gap: var(--space-2);
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1 1 0;
    min-width: 0;
  }
  .lbl {
    font-size: var(--text-2xs);
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: var(--tracking-eyebrow);
  }
  .field input {
    width: 100%;
    padding: var(--space-2);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-primary);
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
  }
  .field input:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-muted);
  }
  .sep {
    color: var(--text-muted);
    padding-bottom: var(--space-2);
  }
</style>
