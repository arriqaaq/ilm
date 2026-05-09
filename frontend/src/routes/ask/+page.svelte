<script lang="ts">
  import type { ApiHadithSearchResult, ApiAyahSearchResult } from '$lib/types';
  import { truncate, stripHtml } from '$lib/utils';
  import { language } from '$lib/stores/language';
  import { appConfig } from '$lib/stores/config';
  import { marked } from 'marked';
  import PageHeader from '$lib/components/common/PageHeader.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import Button from '$lib/components/common/Button.svelte';
  import Ornament from '$lib/components/common/Ornament.svelte';

  marked.setOptions({ breaks: true, gfm: true });

  type SourceMode = 'both' | 'quran' | 'hadith';

  interface NarratorSource {
    id: string;
    name_ar?: string;
    name_en: string;
    generation?: string;
    hadith_count?: number;
    kunya?: string;
    bio?: string;
    death_year?: number;
    teachers?: { id: string; name_ar?: string; name_en: string; generation?: string }[];
    students?: { id: string; name_ar?: string; name_en: string; generation?: string }[];
  }

  interface Message {
    role: 'user' | 'assistant';
    content: string;
    hadith_sources?: ApiHadithSearchResult[];
    quran_sources?: ApiAyahSearchResult[];
    narrator_sources?: NarratorSource[];
    streaming?: boolean;
  }

  let messages: Message[] = $state([]);
  let input = $state('');
  let loading = $state(false);
  let sourceMode: SourceMode = $state('both');
  let chatContainer: HTMLDivElement = $state(null!);

  function scrollToBottom() {
    if (chatContainer) chatContainer.scrollTop = chatContainer.scrollHeight;
  }

  function getEndpoint(): string {
    switch (sourceMode) {
      case 'quran': return '/v1/ask/quran';
      case 'hadith': return '/v1/ask/hadith';
      case 'both': return '/v1/ask/all';
    }
  }

  function getTitle(): string {
    switch (sourceMode) {
      case 'quran': return 'Ask about the Quran';
      case 'hadith': return 'Ask about Hadith';
      case 'both': return 'Ask about Quran & Sunnah';
    }
  }

  function getPlaceholder(): string {
    switch (sourceMode) {
      case 'quran': return 'Ask about the Quran...';
      case 'hadith': return 'Ask about hadiths...';
      case 'both': return 'Ask about Quran & Sunnah...';
    }
  }

  const suggestions: Record<SourceMode, { label: string; text: string }[]> = {
    both: [
      { label: 'Patience', text: 'What do the Quran and Hadith say about patience?' },
      { label: 'Abu Huraira', text: 'How many hadiths did Abu Huraira narrate?' },
      { label: 'Teachers', text: "Who were Imam al-Bukhari's teachers?" },
    ],
    quran: [
      { label: 'Patience', text: 'What does the Quran say about patience?' },
      { label: 'Charity', text: 'What are the verses about charity and giving?' },
      { label: 'Justice', text: 'What does the Quran say about justice?' },
    ],
    hadith: [
      { label: 'Neighbors', text: 'What did the Prophet say about kindness to neighbors?' },
      { label: 'Abu Huraira', text: 'How many hadiths did Abu Huraira narrate?' },
      { label: 'Teachers', text: "Who were Imam al-Bukhari's teachers?" },
    ],
  };

  function switchMode(mode: SourceMode) {
    if (mode !== sourceMode) {
      sourceMode = mode;
      messages = [];
    }
  }

  async function handleSubmit(e: Event) {
    e.preventDefault();
    const question = input.trim();
    if (!question || loading) return;

    input = '';
    messages = [...messages, { role: 'user', content: question }];
    const assistantMsg: Message = { role: 'assistant', content: '', streaming: true };
    messages = [...messages, assistantMsg];
    loading = true;

    try {
      const res = await fetch(getEndpoint(), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question }),
      });

      if (!res.ok) {
        const idx = messages.length - 1;
        messages[idx] = { ...messages[idx], content: `Error: ${res.statusText}`, streaming: false };
        loading = false;
        return;
      }

      const reader = res.body!.getReader();
      const decoder = new TextDecoder();
      let buffer = '';
      const idx = messages.length - 1;

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          if (!line.startsWith('data: ')) continue;
          const jsonStr = line.slice(6).trim();
          if (!jsonStr) continue;
          try {
            const data = JSON.parse(jsonStr);
            // Unified: {quran_sources, hadith_sources, narrator_sources}
            if (data.quran_sources || data.hadith_sources || data.narrator_sources) {
              messages[idx] = {
                ...messages[idx],
                quran_sources: data.quran_sources,
                hadith_sources: data.hadith_sources,
                narrator_sources: data.narrator_sources,
              };
            }
            // Hadith-only: {sources}
            else if (data.sources && sourceMode === 'hadith') {
              messages[idx] = { ...messages[idx], hadith_sources: data.sources };
            }
            // Quran-only: {sources}
            else if (data.sources && sourceMode === 'quran') {
              messages[idx] = { ...messages[idx], quran_sources: data.sources };
            }
            else if (data.text) {
              messages[idx] = { ...messages[idx], content: messages[idx].content + data.text };
              scrollToBottom();
            } else if (data.done) {
              messages[idx] = { ...messages[idx], streaming: false };
            } else if (data.error) {
              messages[idx] = { ...messages[idx], content: messages[idx].content + `\n\n[Error: ${data.error}]`, streaming: false };
            }
          } catch { /* skip */ }
        }
      }
      messages[idx] = { ...messages[idx], streaming: false };
    } catch (e: any) {
      const idx = messages.length - 1;
      messages[idx] = { ...messages[idx], content: `Error: ${e.message}`, streaming: false };
    } finally {
      loading = false;
      scrollToBottom();
    }
  }
</script>

{#if !$appConfig.advanced_enabled}
<div class="page-shell">
  <PageHeader eyebrow="Chat" title="Ask the Library" />
  <p class="unavailable-msg">Advanced features are not available in this build.</p>
</div>
{:else}
<div class="ask-page">
  <header class="ask-header">
    <div class="header-inner">
      <Eyebrow>Chat · {getTitle()}</Eyebrow>
      <div class="mode-toggle">
        <button class="mode-btn" class:active={sourceMode === 'both'} onclick={() => switchMode('both')}>Both</button>
        <button class="mode-btn" class:active={sourceMode === 'quran'} onclick={() => switchMode('quran')}>Qurʾān</button>
        <button class="mode-btn" class:active={sourceMode === 'hadith'} onclick={() => switchMode('hadith')}>Ḥadīth</button>
      </div>
    </div>
  </header>

  <div class="chat-container" bind:this={chatContainer}>
    {#if messages.length === 0}
      <div class="empty-state">
        <Ornament variant="star" size={28} color="var(--accent)" />
        <h2 class="empty-title">{getTitle()}</h2>
        <p class="empty-hint">Answers are grounded in {sourceMode === 'both' ? 'Qurʾānic verses, Tafsīr, and Ḥadīth' : sourceMode === 'quran' ? 'Qurʾānic verses and Tafsīr Ibn Kathīr' : 'hadith texts'} using semantic search.</p>
        <div class="suggestions">
          <div class="suggestion-eyebrow"><Eyebrow tone="muted">Try asking</Eyebrow></div>
          <div class="suggestion-row">
            {#each suggestions[sourceMode] as s}
              <button class="suggestion" onclick={() => { input = s.text; }}>{s.label}</button>
            {/each}
          </div>
        </div>
      </div>
    {/if}

    {#each messages as msg}
      <article class="message {msg.role}">
        <div class="role-label-wrap">
          <Eyebrow tone={msg.role === 'user' ? 'accent' : 'muted'}>{msg.role === 'user' ? 'You' : 'Assistant'}</Eyebrow>
        </div>
        <div class="message-content">
          {#if msg.role === 'assistant'}
            <div class="assistant-text prose">{@html marked(msg.content)}{#if msg.streaming}<span class="cursor">|</span>{/if}</div>

            {#if msg.quran_sources && msg.quran_sources.length > 0}
              <details class="sources">
                <summary>Quran Sources ({msg.quran_sources.length} ayahs)</summary>
                <div class="source-list">
                  {#each msg.quran_sources as s}
                    <a href="/quran/{s.surah_number}?ayah={s.ayah_number}" class="source-card">
                      <span class="source-ref mono quran-ref">{s.surah_number}:{s.ayah_number}</span>
                      <span class="source-arabic" dir="rtl">{truncate(s.text_ar, 80)}</span>
                      {#if s.text_en}<span class="source-text">{truncate(s.text_en, 120)}</span>{/if}
                    </a>
                  {/each}
                </div>
              </details>
            {/if}

            {#if msg.hadith_sources && msg.hadith_sources.length > 0}
              <details class="sources">
                <summary>Hadith Sources ({msg.hadith_sources.length} hadiths)</summary>
                <div class="source-list">
                  {#each msg.hadith_sources as s}
                    <a href="/hadiths/{s.id}" class="source-card">
                      <span class="source-num mono">#{s.hadith_number}</span>
                      {#if s.narrator_text}<span class="source-narrator">{s.narrator_text}</span>{/if}
                      <span class="source-text">{$language === 'en' && s.text_en ? truncate(stripHtml(s.text_en), 120) : truncate(s.text_ar || stripHtml(s.text_en ?? ''), 120)}</span>
                    </a>
                  {/each}
                </div>
              </details>
            {/if}

            {#if msg.narrator_sources && msg.narrator_sources.length > 0}
              <details class="sources" open>
                <summary>Narrator Sources ({msg.narrator_sources.length})</summary>
                <div class="source-list">
                  {#each msg.narrator_sources as n}
                    <a href="/narrators/{n.id}" class="source-card narrator-card">
                      <div class="narrator-header">
                        <span class="source-narrator">{n.name_en ?? n.name_ar ?? n.id}</span>
                        {#if n.name_ar}<span class="source-arabic" dir="rtl">{n.name_ar}</span>{/if}
                      </div>
                      <div class="narrator-meta">
                        {#if n.generation}<span class="narrator-tag">Gen {n.generation}</span>{/if}
                        {#if n.hadith_count}<span class="narrator-tag">{n.hadith_count} hadiths</span>{/if}
                        {#if n.death_year}<span class="narrator-tag">d. {n.death_year} AH</span>{/if}
                      </div>
                    </a>
                  {/each}
                </div>
              </details>
            {/if}
          {:else}
            <div class="user-text">{msg.content}</div>
          {/if}
        </div>
      </article>
    {/each}
  </div>

  <form class="input-area" onsubmit={handleSubmit}>
    <input type="text" placeholder={getPlaceholder()} bind:value={input} disabled={loading} class="chat-input" />
    <Button type="submit" variant="primary" size="md" disabled={loading || !input.trim()}>
      {loading ? '…' : 'Send'}
    </Button>
  </form>
</div>
{/if}

<style>
  .ask-page {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
  }
  .unavailable-msg {
    color: var(--text-secondary);
    font-family: var(--font-serif);
    font-size: var(--text-body);
    font-style: italic;
  }

  .ask-header {
    padding: var(--space-3) var(--space-6);
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    flex-shrink: 0;
  }
  .header-inner {
    max-width: 760px;
    margin: 0 auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
  }
  .mode-toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .mode-btn {
    padding: var(--space-2) var(--space-4);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-medium);
    background: transparent;
    color: var(--text-secondary);
    border: none;
    cursor: pointer;
    transition: all var(--transition);
  }
  .mode-btn.active {
    background: var(--accent-muted);
    color: var(--accent);
  }
  .mode-btn:hover:not(.active) { background: var(--bg-hover); }

  .chat-container {
    flex: 1;
    overflow-y: auto;
    padding: var(--space-8) var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    max-width: 760px;
    margin: 0 auto;
    width: 100%;
  }
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    text-align: center;
    color: var(--text-secondary);
    gap: var(--space-3);
    padding: var(--space-12) var(--space-6);
  }
  .empty-title {
    font-family: var(--font-serif);
    font-size: var(--text-lead);
    font-weight: var(--font-weight-semibold);
    color: var(--text-primary);
    margin: var(--space-2) 0 0;
  }
  .empty-hint {
    max-width: 480px;
    line-height: 1.7;
    font-family: var(--font-serif);
    font-size: var(--text-body);
    color: var(--text-muted);
    margin: 0;
  }
  .suggestions {
    margin-top: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    align-items: center;
  }
  .suggestion-eyebrow { margin-bottom: var(--space-1); }
  .suggestion-row {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
    justify-content: center;
  }
  .suggestion {
    padding: var(--space-1) var(--space-3);
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    color: var(--text-secondary);
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    transition: all var(--transition);
    cursor: pointer;
  }
  .suggestion:hover {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-muted);
  }

  .message { width: 100%; }
  .role-label-wrap { margin-bottom: var(--space-2); }
  .user-text {
    font-family: var(--font-serif);
    font-size: var(--text-body);
    line-height: 1.7;
    color: var(--text-primary);
  }
  .assistant-text {
    font-family: var(--font-serif);
    font-size: var(--text-body);
    line-height: 1.75;
    color: var(--text-primary);
  }
  .assistant-text :global(p) { margin: 0.6em 0; }
  .assistant-text :global(strong) { font-weight: var(--font-weight-semibold); color: var(--text-primary); }
  .assistant-text :global(em) { font-style: italic; }
  .assistant-text :global(ul), .assistant-text :global(ol) { margin: 0.6em 0; padding-left: 1.5em; }
  .assistant-text :global(li) { margin: 0.3em 0; }
  .assistant-text :global(h1),
  .assistant-text :global(h2),
  .assistant-text :global(h3) {
    margin: 0.9em 0 0.3em;
    font-family: var(--font-serif);
    font-weight: var(--font-weight-semibold);
  }
  .assistant-text :global(h2) { font-size: var(--text-lg); }
  .assistant-text :global(h3) { font-size: var(--text-base); }
  .assistant-text :global(code) {
    background: var(--bg-hover);
    padding: 2px var(--space-1);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
    font-size: 0.92em;
  }
  .assistant-text :global(blockquote) {
    border-left: 2px solid var(--accent);
    margin: 0.6em 0;
    padding: 0.25em var(--space-3);
    color: var(--text-secondary);
    font-style: italic;
  }
  .cursor { animation: blink 1s step-end infinite; color: var(--accent); }
  @keyframes blink { 50% { opacity: 0; } }

  .sources {
    margin-top: var(--space-4);
    border-top: 1px solid var(--border-subtle);
    padding-top: var(--space-3);
  }
  .sources summary {
    font-family: var(--font-sans);
    font-size: var(--text-meta);
    color: var(--text-muted);
    cursor: pointer;
  }
  .sources summary:hover { color: var(--accent); }
  .source-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .source-card {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-3);
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: var(--radius);
    color: var(--text-primary);
    font-size: var(--text-meta);
    text-decoration: none;
    transition: all var(--transition);
  }
  .source-card:hover {
    border-color: var(--accent);
    background: var(--bg-hover);
    color: var(--text-primary);
  }
  .source-ref {
    font-size: var(--text-meta);
    font-weight: var(--font-weight-semibold);
  }
  .quran-ref { color: var(--success); }
  .source-num {
    color: var(--text-muted);
    font-size: var(--text-meta);
  }
  .source-narrator {
    color: var(--accent);
    font-size: var(--text-meta);
    font-family: var(--font-serif);
    font-style: italic;
  }
  .source-arabic {
    color: var(--text-primary);
    font-size: 1rem;
  }
  .source-text { color: var(--text-secondary); }
  .narrator-card { gap: var(--space-1); }
  .narrator-header {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
  }
  .narrator-meta {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }
  .narrator-tag {
    font-size: var(--text-2xs);
    padding: 2px var(--space-2);
    background: var(--bg-secondary);
    border-radius: var(--radius-pill);
    color: var(--text-muted);
    white-space: nowrap;
  }

  .input-area {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-4) var(--space-6);
    border-top: 1px solid var(--border-subtle);
    background: var(--bg-surface);
    max-width: 760px;
    margin: 0 auto;
    width: 100%;
    flex-shrink: 0;
  }
  .chat-input {
    flex: 1;
    padding: var(--space-3) var(--space-4);
    font-family: var(--font-sans);
    font-size: var(--text-body);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-primary);
    color: var(--text-primary);
    outline: none;
  }
  .chat-input:focus { border-color: var(--accent); }

  @media (max-width: 640px) {
    .ask-header { padding: var(--space-2) var(--space-3); }
    .mode-btn { padding: var(--space-1) var(--space-3); font-size: var(--text-meta); }
    .chat-container { padding: var(--space-5) var(--space-4); }
    .input-area { padding: var(--space-3); }
  }
</style>
