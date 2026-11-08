<script lang="ts">
  import { CornerDownLeft, Puzzle } from "@lucide/svelte";
  import { api, errorMessage } from "../../lib/api";
  import { matchCommands, parseSlash, type SlashCommand } from "../../lib/slash";
  import { workspace } from "../../lib/stores/workspace.svelte";
  import { plugins } from "../../lib/stores/plugins.svelte";

  let { conversationId }: { conversationId: string } = $props();

  const MAX = 4000;
  let text = $state("");
  let area = $state<HTMLTextAreaElement>();
  let busy = $state(false);
  let selected = $state(0);
  let dismissed = $state(false);

  const conv = $derived(workspace.conversations.find((c) => c.id === conversationId));
  const canSend = $derived(workspace.capabilities.sendMessages);
  const suggestions = $derived(dismissed ? [] : matchCommands(text, plugins.contributions.commands));
  const remaining = $derived(MAX - [...text].length);

  $effect(() => {
    void conversationId;
    queueMicrotask(() => area?.focus());
  });

  $effect(() => {
    void suggestions.length;
    if (selected >= suggestions.length) selected = 0;
  });

  function autosize() {
    if (!area) return;
    area.style.height = "auto";
    area.style.height = `${Math.min(area.scrollHeight, window.innerHeight * 0.4)}px`;
  }

  $effect(() => {
    void text;
    autosize();
  });

  function complete(c: SlashCommand) {
    text = `/${c.name} `;
    dismissed = true;
    area?.focus();
  }

  async function runCommand(cmd: SlashCommand, args: string) {
    try {
      const out = await api.pluginRunCommand(cmd.pluginId, cmd.name, args);
      if (out.replaceWith !== null && out.replaceWith !== undefined) {
        text = out.replaceWith;
      } else {
        text = "";
      }
      if (out.ephemeral) workspace.addNote(conversationId, { tone: "info", source: `/${cmd.name}`, text: out.ephemeral });
    } catch (e) {
      workspace.addNote(conversationId, { tone: "error", source: `/${cmd.name} (${cmd.pluginName})`, text: errorMessage(e) });
    }
  }

  async function submit() {
    const value = text;
    if (!value.trim() || busy || !canSend) return;
    const parsed = parseSlash(value);
    const cmd = parsed && plugins.contributions.commands.find((c) => c.name === parsed.name);
    busy = true;
    try {
      if (parsed && cmd) {
        await runCommand(cmd, parsed.args);
      } else if (remaining >= 0) {
        if (await workspace.send(conversationId, value)) text = "";
      }
    } finally {
      busy = false;
      dismissed = false;
      area?.focus();
    }
  }

  function onKey(e: KeyboardEvent) {
    if (suggestions.length) {
      if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        e.preventDefault();
        const d = e.key === "ArrowDown" ? 1 : -1;
        selected = (selected + d + suggestions.length) % suggestions.length;
        return;
      }
      if (e.key === "Tab" || (e.key === "Enter" && !e.shiftKey)) {
        const pick = suggestions[selected];
        if (pick && text.trim() !== `/${pick.name}`) {
          e.preventDefault();
          complete(pick);
          return;
        }
      }
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        dismissed = true;
        return;
      }
    }
    if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      void submit();
    }
  }
</script>

<div class="composer-wrap">
  {#if suggestions.length}
    <div class="suggest" role="listbox" aria-label="Commands" id="dz-suggest">
      <div class="suggest-head">Commands from plugins</div>
      {#each suggestions as s, i (s.pluginId + s.name)}
        <button
          role="option"
          aria-selected={i === selected}
          class="opt"
          class:sel={i === selected}
          onmousedown={(e) => {
            e.preventDefault();
            complete(s);
          }}
          onmouseenter={() => (selected = i)}
        >
          <span class="cmd">/{s.name}</span>
          <span class="desc">{s.description}</span>
          <span class="from"><Puzzle size={12} /> {s.pluginName}</span>
        </button>
      {/each}
    </div>
  {/if}

  <div class="composer" class:disabled={!canSend}>
    <label class="sr-only" for="dz-composer">Message {conv ? `#${conv.name}` : ""}</label>
    <textarea
      id="dz-composer"
      bind:this={area}
      bind:value={text}
      rows="1"
      placeholder={canSend ? `Message #${conv?.name ?? ""}` : "This integration can't send messages"}
      disabled={!canSend}
      onkeydown={onKey}
      oninput={() => (dismissed = false)}
      aria-autocomplete="list"
      aria-controls={suggestions.length ? "dz-suggest" : undefined}
      spellcheck="true"
    ></textarea>
    <div class="meta">
      {#if remaining < 500}
        <span class="count num" class:over={remaining < 0}>{remaining}</span>
      {/if}
      <button class="send" aria-label="Send message" disabled={!canSend || !text.trim() || busy || remaining < 0} onclick={submit}>
        <CornerDownLeft size={16} strokeWidth={2} />
      </button>
    </div>
  </div>
  <div class="hint">
    <span><kbd>Enter</kbd> send</span>
    <span><kbd>Shift Enter</kbd> new line</span>
    {#if plugins.contributions.commands.length}<span><kbd>/</kbd> plugin commands</span>{/if}
  </div>
</div>

<style>
  .composer-wrap {
    position: relative;
    padding: 4px 16px 10px;
    flex: none;
  }
  .composer {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    padding: 8px 8px 8px 14px;
    border: 1px solid var(--dz-border);
    border-radius: 10px;
    background: var(--dz-input);
    transition: border-color var(--dz-fast) var(--dz-ease), box-shadow var(--dz-fast) var(--dz-ease);
  }
  .composer:focus-within {
    border-color: var(--dz-border-strong);
    box-shadow: 0 0 0 3px rgba(var(--dz-accent-rgb), 0.1);
  }
  .composer.disabled {
    opacity: 0.6;
  }
  textarea {
    flex: 1;
    min-height: 24px;
    max-height: 40vh;
    padding: 3px 0;
    border: 0;
    outline: none;
    resize: none;
    background: none;
    line-height: 1.45;
  }
  textarea::placeholder {
    color: var(--dz-text-muted);
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .count {
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
  .count.over {
    color: var(--dz-danger);
    font-weight: 650;
  }
  .send {
    display: grid;
    place-items: center;
    width: 30px;
    height: 30px;
    border-radius: var(--dz-r-control);
    background: var(--dz-accent);
    color: var(--dz-on-accent);
    transition: background-color var(--dz-fast) var(--dz-ease), opacity var(--dz-fast) var(--dz-ease);
  }
  .send:hover:not(:disabled) {
    background: var(--dz-accent-hover);
  }
  .send:disabled {
    background: var(--dz-hover);
    color: var(--dz-text-muted);
  }
  .hint {
    display: flex;
    gap: 14px;
    padding: 6px 2px 0;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
  .hint kbd {
    margin-right: 4px;
    font-size: 10.5px;
  }
  .suggest {
    position: absolute;
    left: 16px;
    right: 16px;
    bottom: calc(100% - 4px);
    padding: 6px;
    border-radius: var(--dz-r-float);
    background: var(--dz-surface-elevated);
    box-shadow: var(--dz-shadow-float);
    z-index: 10;
  }
  .suggest-head {
    padding: 4px 8px 6px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    font-weight: 600;
  }
  .opt {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    padding: 7px 8px;
    border-radius: var(--dz-r-control);
    text-align: left;
  }
  .opt.sel {
    background: var(--dz-active);
  }
  .cmd {
    font-weight: 620;
    min-width: 96px;
  }
  .desc {
    flex: 1;
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .from {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
</style>
