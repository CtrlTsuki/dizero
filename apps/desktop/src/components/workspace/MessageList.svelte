<script lang="ts">
  import { tick } from "svelte";
  import { ArrowDown, Hash, X } from "@lucide/svelte";
  import MessageRow from "./MessageRow.svelte";
  import { workspace } from "../../lib/stores/workspace.svelte";
  import { app } from "../../lib/stores/app.svelte";
  import { dayLabel, sameGroup } from "../../lib/format";

  let { conversationId }: { conversationId: string } = $props();

  let scroller = $state<HTMLDivElement>();
  let sentinel = $state<HTMLDivElement>();
  let stuck = true;
  let unseen = $state(0);
  let lastCount = 0;
  let lastFirstId: string | undefined;
  let heightBefore = 0;
  let topBefore = 0;

  const st = $derived(workspace.convs[conversationId]);
  const conv = $derived(workspace.conversations.find((c) => c.id === conversationId));
  const typing = $derived(workspace.typing[conversationId] ?? []);
  const notes = $derived(workspace.notes[conversationId] ?? []);
  const selfName = $derived(workspace.self?.displayName ?? "");
  const showAvatars = $derived(app.settings?.appearance.showAvatars ?? true);

  /** Rows with day dividers and grouping precomputed. */
  const rows = $derived.by(() => {
    const msgs = st?.messages ?? [];
    return msgs.map((m, i) => {
      const prev = msgs[i - 1];
      const newDay = !prev || new Date(prev.createdAt).toDateString() !== new Date(m.createdAt).toDateString();
      const continued =
        !!prev && !newDay && sameGroup({ authorId: prev.author.id, createdAt: prev.createdAt }, { authorId: m.author.id, createdAt: m.createdAt });
      return { m, newDay, continued };
    });
  });

  // Capture scroll geometry before the DOM changes…
  $effect.pre(() => {
    void rows.length;
    void typing.length;
    void notes.length;
    if (!scroller) return;
    heightBefore = scroller.scrollHeight;
    topBefore = scroller.scrollTop;
  });

  // …then keep the view anchored after it changes.
  $effect(() => {
    const msgs = st?.messages ?? [];
    void typing.length;
    void notes.length;
    if (!scroller) return;
    const firstId = msgs[0]?.id;
    const prepended = lastFirstId !== undefined && firstId !== lastFirstId && msgs.length >= lastCount;
    const appended = msgs.length > lastCount && !prepended;
    if (prepended && !stuck) {
      scroller.scrollTop = topBefore + (scroller.scrollHeight - heightBefore);
    } else if (stuck) {
      scroller.scrollTop = scroller.scrollHeight;
    } else if (appended) {
      const fresh = msgs.slice(lastCount).filter((m) => !m.author.isSelf).length;
      unseen += fresh;
      if (msgs.slice(lastCount).some((m) => m.author.isSelf)) scrollToBottom();
    }
    lastCount = msgs.length;
    lastFirstId = firstId;
  });

  function onScroll() {
    if (!scroller) return;
    stuck = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 80;
    if (stuck) unseen = 0;
  }

  function scrollToBottom() {
    if (!scroller) return;
    scroller.scrollTop = scroller.scrollHeight;
    stuck = true;
    unseen = 0;
  }

  async function present() {
    await workspace.jumpToPresent(conversationId);
    await tick();
    scrollToBottom();
  }

  // Load older history when the top sentinel scrolls into view.
  $effect(() => {
    if (!sentinel || !scroller) return;
    const io = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting) && st?.hasMore && !st.loadingOlder) {
          stuck = false;
          void workspace.loadOlder(conversationId);
        }
      },
      { root: scroller, rootMargin: "300px 0px 0px 0px" },
    );
    io.observe(sentinel);
    return () => io.disconnect();
  });
</script>

<div class="list" bind:this={scroller} onscroll={onScroll} role="log" aria-live="polite" aria-relevant="additions" tabindex="-1">
  <div class="spacer"></div>
  <div bind:this={sentinel} class="sentinel" aria-hidden="true"></div>

  {#if st && !st.hasMore && conv}
    <div class="intro">
      <div class="intro-glyph"><Hash size={26} strokeWidth={1.7} /></div>
      <h2>This is the start of #{conv.name}</h2>
      {#if conv.topic}<p>{conv.topic}</p>{/if}
    </div>
  {:else if st?.loadingOlder}
    <div class="loading-older" aria-live="polite">Loading older messages…</div>
  {/if}

  {#if !st}
    <div class="skel" aria-busy="true">
      {#each [0.5, 0.75, 0.35] as w, i (i)}
        <div class="sk-row">
          <div class="skeleton" style:width="36px" style:height="36px" style:border-radius="50%"></div>
          <div class="sk-lines">
            <div class="skeleton" style:width="{w * 30}%" style:height="11px"></div>
            <div class="skeleton" style:width="{w * 100}%" style:height="11px"></div>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  {#each rows as r (r.m.id)}
    {#if r.newDay}
      <div class="day" role="separator"><span>{dayLabel(r.m.createdAt)}</span></div>
    {/if}
    <MessageRow
      message={r.m}
      continued={r.continued}
      {selfName}
      {showAvatars}
      highlighted={workspace.highlightId === r.m.id}
    />
  {/each}

  {#each notes as n (n.id)}
    <div class="note" class:error={n.tone === "error"} role="status">
      <span class="note-source">{n.source}</span>
      <span class="note-text">{n.text}</span>
      <span class="note-hint">Only you can see this</span>
      <button class="icon-btn" aria-label="Dismiss" onclick={() => workspace.dismissNote(conversationId, n.id)}>
        <X size={14} />
      </button>
    </div>
  {/each}

  <div class="typing" aria-live="polite">
    {#if typing.length}
      <span class="dots" aria-hidden="true"><i></i><i></i><i></i></span>
      <span><strong>{typing.map((t) => t.name).join(", ")}</strong> {typing.length > 1 ? "are" : "is"} typing…</span>
    {/if}
  </div>
</div>

{#if st?.detached}
  <div class="banner">
    <span>You're viewing older messages{st.newWhileDetached ? `, ${st.newWhileDetached} new since` : ""}.</span>
    <button class="btn btn-sm btn-primary" onclick={present}>Jump to present</button>
  </div>
{:else if unseen > 0}
  <button class="pill" onclick={scrollToBottom}>
    <ArrowDown size={14} strokeWidth={2.2} />
    {unseen} new message{unseen > 1 ? "s" : ""}
  </button>
{/if}

<style>
  .list {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-anchor: none;
    display: flex;
    flex-direction: column;
    padding-bottom: 4px;
  }
  .spacer {
    flex: 1;
  }
  .sentinel {
    height: 1px;
  }
  .intro {
    padding: 24px 24px 8px 72px;
  }
  .intro-glyph {
    display: grid;
    place-items: center;
    width: 52px;
    height: 52px;
    margin-bottom: 12px;
    border-radius: 50%;
    background: var(--dz-surface);
    border: 1px solid var(--dz-border);
    color: var(--dz-text-secondary);
  }
  .intro h2 {
    font-size: var(--dz-text-xl);
    margin-bottom: 4px;
  }
  .intro p {
    color: var(--dz-text-muted);
  }
  .loading-older {
    padding: 14px 72px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-sm);
  }
  .skel {
    display: grid;
    gap: 22px;
    padding: 16px;
  }
  .sk-row {
    display: flex;
    gap: 14px;
  }
  .sk-lines {
    flex: 1;
    display: grid;
    gap: 8px;
  }
  .day {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 22px 16px 4px 72px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    font-weight: 600;
  }
  .day::after {
    content: "";
    flex: 1;
    height: 1px;
    background: var(--dz-border);
  }
  .note {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 10px 16px 0 72px;
    padding: 8px 6px 8px 12px;
    border: 1px dashed var(--dz-border-strong);
    border-radius: var(--dz-r-control);
    font-size: var(--dz-text-sm);
  }
  .note.error {
    border-color: color-mix(in srgb, var(--dz-danger) 50%, transparent);
  }
  .note-source {
    font-weight: 620;
    white-space: nowrap;
  }
  .note.error .note-source {
    color: var(--dz-danger);
  }
  .note-text {
    flex: 1;
    min-width: 0;
    color: var(--dz-text-secondary);
    overflow-wrap: anywhere;
  }
  .note-hint {
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
    white-space: nowrap;
  }
  .note .icon-btn {
    width: 24px;
    height: 24px;
  }
  .typing {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 22px;
    padding: 2px 16px 0 72px;
    color: var(--dz-text-muted);
    font-size: var(--dz-text-xs);
  }
  .typing strong {
    color: var(--dz-text-secondary);
    font-weight: 620;
  }
  .dots {
    display: inline-flex;
    gap: 3px;
  }
  .dots i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--dz-text-secondary);
    animation: dz-blink 1.2s infinite ease-in-out;
  }
  .dots i:nth-child(2) {
    animation-delay: 0.15s;
  }
  .dots i:nth-child(3) {
    animation-delay: 0.3s;
  }
  @keyframes dz-blink {
    0%,
    80%,
    100% {
      opacity: 0.25;
    }
    40% {
      opacity: 1;
    }
  }
  .pill {
    position: absolute;
    left: 50%;
    bottom: 96px;
    transform: translateX(-50%);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 30px;
    padding: 0 12px;
    border-radius: 999px;
    background: var(--dz-accent);
    color: var(--dz-on-accent);
    font-size: var(--dz-text-sm);
    font-weight: 600;
    box-shadow: var(--dz-shadow-float);
    z-index: 4;
  }
  .banner {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin: 0 16px;
    padding: 8px 8px 8px 14px;
    border: 1px solid var(--dz-border);
    border-bottom: 0;
    border-radius: var(--dz-r-control) var(--dz-r-control) 0 0;
    background: var(--dz-surface);
    color: var(--dz-text-secondary);
    font-size: var(--dz-text-sm);
  }
</style>
