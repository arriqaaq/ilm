<script lang="ts">
  import type { ApiAyahSearchResult } from '$lib/types';
  import { truncate } from '$lib/utils';
  import { appConfig } from '$lib/stores/config';
  import { marked } from 'marked';
  import PageHeader from '$lib/components/common/PageHeader.svelte';
  import Eyebrow from '$lib/components/common/Eyebrow.svelte';
  import Button from '$lib/components/common/Button.svelte';
  import Ornament from '$lib/components/common/Ornament.svelte';

  marked.setOptions({ breaks: true, gfm: true });

  interface Message {
    role: 'user' | 'assistant';
    content: string;
    sources?: ApiAyahSearchResult[];
    streaming?: boolean;
  }

  let messages: Message[] = $state([]);
  let input = $state('');
  let loading = $state(false);
  let chatContainer: HTMLDivElement = $state(null!);

  function scrollToBottom() {
    if (chatContainer) chatContainer.scrollTop = chatContainer.scrollHeight;
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
      const res = await fetch('/v1/ask/quran', {
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
            if (data.sources) {
              messages[idx] = { ...messages[idx], sources: data.sources };
            } else if (data.text) {
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
  <PageHeader eyebrow="Qurʾān · Chat" title="Ask about the Qurʾān" />
  <p class="unavailable-msg">Advanced features are not available in this build.</p>
</div>
{:else}
<div class="ask-page">
  <header class="ask-header">
    <div class="header-inner">
      <Eyebrow>Qurʾān · Chat</Eyebrow>
    </div>
  </header>

  <div class="chat-container" bind:this={chatContainer}>
    {#if messages.length === 0}
      <div class="empty-state">
        <Ornament variant="star" size={28} color="var(--accent)" />
        <h2 class="empty-title">Ask about the Qurʾān</h2>
        <p class="empty-hint">Answers are grounded in Qurʾānic verses and Tafsīr Ibn Kathīr using semantic search.</p>
        <div class="suggestions">
          <div class="suggestion-eyebrow"><Eyebrow tone="muted">Try asking</Eyebrow></div>
          <div class="suggestion-row">
            <button class="suggestion" onclick={() => { input = 'What does the Qurʾān say about patience?'; }}>Patience</button>
            <button class="suggestion" onclick={() => { input = 'What are the verses about charity and giving?'; }}>Charity</button>
            <button class="suggestion" onclick={() => { input = 'What does the Qurʾān say about justice?'; }}>Justice</button>
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
            {#if msg.sources && msg.sources.length > 0}
              <details class="sources">
                <summary>Sources ({msg.sources.length} ayāt)</summary>
                <div class="source-list">
                  {#each msg.sources as s}
                    <a href="/quran/{s.surah_number}" class="source-card">
                      <span class="source-ref mono">{s.surah_number}:{s.ayah_number}</span>
                      <span class="source-arabic arabic-prose" dir="rtl">{truncate(s.text_ar, 80)}</span>
                      {#if s.text_en}<span class="source-text">{truncate(s.text_en, 120)}</span>{/if}
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
    <input type="text" placeholder="Ask about the Qurʾān…" bind:value={input} disabled={loading} class="chat-input" />
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
  }

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
  .assistant-text :global(h2) {
    font-family: var(--font-serif);
    font-size: var(--text-lg);
    margin: 0.9em 0 0.3em;
    font-weight: var(--font-weight-semibold);
  }
  .assistant-text :global(h3) {
    font-family: var(--font-serif);
    font-size: var(--text-base);
    margin: 0.9em 0 0.3em;
    font-weight: var(--font-weight-semibold);
  }
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
    color: var(--accent);
    font-size: var(--text-meta);
    font-weight: var(--font-weight-semibold);
  }
  .source-arabic {
    color: var(--text-primary);
    font-size: 1rem;
  }
  .source-text {
    color: var(--text-secondary);
    font-family: var(--font-serif);
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
</style>
