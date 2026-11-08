import { api, errorMessage } from "../api";
import type { Capabilities } from "../bindings/Capabilities";
import type { Community } from "../bindings/Community";
import type { Conversation } from "../bindings/Conversation";
import type { IntegrationInfo } from "../bindings/IntegrationInfo";
import type { Member } from "../bindings/Member";
import type { Message } from "../bindings/Message";
import type { SearchHit } from "../bindings/SearchHit";
import type { User } from "../bindings/User";
import { mergeWindow } from "../messageWindow";
import { app } from "./app.svelte";
import { toasts } from "./toasts.svelte";

export interface ConvState {
  messages: Message[];
  hasMore: boolean;
  loaded: boolean;
  loadingOlder: boolean;
  /** Newest messages were trimmed while paging back; live updates are paused. */
  detached: boolean;
  newWhileDetached: number;
}

export interface LocalNote {
  id: string;
  tone: "info" | "error";
  source: string;
  text: string;
  createdAt: number;
}

export interface Typist {
  userId: string;
  name: string;
}

const NO_CAPS: Capabilities = {
  sendMessages: false,
  deleteOwnMessages: false,
  editMessages: false,
  attachmentsUpload: false,
  reactions: false,
  presence: false,
  typingIndicators: false,
  search: false,
  voice: false,
};

let noteSeq = 0;
let persistTimer: ReturnType<typeof setTimeout> | undefined;

class Workspace {
  communities = $state<Community[]>([]);
  conversations = $state<Conversation[]>([]);
  integrations = $state<IntegrationInfo[]>([]);
  self = $state<User | null>(null);
  activeCommunityId = $state<string | null>(null);
  activeConversationId = $state<string | null>(null);
  convs = $state<Record<string, ConvState>>({});
  members = $state<Record<string, Member[]>>({});
  typing = $state<Record<string, Typist[]>>({});
  notes = $state<Record<string, LocalNote[]>>({});
  highlightId = $state<string | null>(null);
  loading = $state(true);
  searchQuery = $state("");
  searchResults = $state<SearchHit[]>([]);
  searching = $state(false);
  #lastConversationByCommunity: Record<string, string> = {};

  activeCommunity = $derived(this.communities.find((c) => c.id === this.activeCommunityId) ?? null);
  activeConversation = $derived(this.conversations.find((c) => c.id === this.activeConversationId) ?? null);
  activeConv = $derived(this.activeConversationId ? (this.convs[this.activeConversationId] ?? null) : null);
  capabilities = $derived.by<Capabilities>(() => {
    const integration = this.integrations.find((i) => i.id === this.activeCommunity?.integrationId);
    return integration?.capabilities ?? NO_CAPS;
  });

  conversationsOf(communityId: string): Conversation[] {
    return this.conversations.filter((c) => c.communityId === communityId);
  }

  grouped(communityId: string): { category: string; items: Conversation[] }[] {
    const out: { category: string; items: Conversation[] }[] = [];
    for (const c of this.conversationsOf(communityId)) {
      const cat = c.category ?? "Channels";
      let g = out.find((x) => x.category === cat);
      if (!g) out.push((g = { category: cat, items: [] }));
      g.items.push(c);
    }
    return out;
  }

  communityUnread(communityId: string): { unread: number; mentions: number } {
    let unread = 0;
    let mentions = 0;
    for (const c of this.conversationsOf(communityId)) {
      unread += c.unreadCount;
      mentions += c.mentionCount;
    }
    return { unread, mentions };
  }

  async init() {
    this.loading = true;
    const snap = await api.workspaceSnapshot();
    this.communities = snap.communities;
    this.conversations = snap.conversations;
    this.integrations = snap.integrations;
    this.self = snap.selfUser;
    const ws = app.settings?.workspace;
    const conv = this.conversations.find((c) => c.id === ws?.lastConversation);
    const communityId = conv?.communityId ?? ws?.lastCommunity ?? this.communities[0]?.id ?? null;
    this.loading = false;
    if (communityId) await this.selectCommunity(communityId, conv?.id);
  }

  async refreshSnapshot() {
    const snap = await api.workspaceSnapshot();
    this.communities = snap.communities;
    this.conversations = snap.conversations;
    this.integrations = snap.integrations;
    this.self = snap.selfUser;
  }

  async selectCommunity(id: string, conversationId?: string) {
    this.activeCommunityId = id;
    const target =
      conversationId ??
      this.#lastConversationByCommunity[id] ??
      this.conversationsOf(id).find((c) => c.unreadCount > 0)?.id ??
      this.conversationsOf(id).find((c) => c.kind === "text")?.id ??
      this.conversationsOf(id)[0]?.id;
    if (!this.members[id]) {
      api.members(id).then((m) => (this.members[id] = m)).catch(() => {});
    }
    if (target) await this.selectConversation(target);
  }

  async selectConversation(id: string) {
    const conv = this.conversations.find((c) => c.id === id);
    if (!conv) return;
    if (conv.communityId !== this.activeCommunityId) {
      this.activeCommunityId = conv.communityId;
      if (!this.members[conv.communityId]) {
        api.members(conv.communityId).then((m) => (this.members[conv.communityId] = m)).catch(() => {});
      }
    }
    this.activeConversationId = id;
    this.#lastConversationByCommunity[conv.communityId] = id;
    this.#persistLocation();
    await this.ensureLoaded(id);
    this.markRead(id);
  }

  /** Move to the previous/next conversation in the active community. */
  async step(delta: 1 | -1) {
    if (!this.activeCommunityId) return;
    const list = this.conversationsOf(this.activeCommunityId);
    const i = list.findIndex((c) => c.id === this.activeConversationId);
    const next = list[(i + delta + list.length) % list.length];
    if (next) await this.selectConversation(next.id);
  }

  #persistLocation() {
    clearTimeout(persistTimer);
    persistTimer = setTimeout(() => {
      const community = this.activeCommunityId;
      const conversation = this.activeConversationId;
      const ws = app.settings?.workspace;
      if (ws && ws.lastCommunity === community && ws.lastConversation === conversation) return;
      app.update((s) => {
        s.workspace.lastCommunity = community;
        s.workspace.lastConversation = conversation;
      });
    }, 1200);
  }

  async ensureLoaded(id: string) {
    if (this.convs[id]?.loaded) return;
    try {
      const page = await api.messagesList(id);
      this.convs[id] = {
        messages: page.messages,
        hasMore: page.hasMore,
        loaded: true,
        loadingOlder: false,
        detached: false,
        newWhileDetached: 0,
      };
    } catch (e) {
      toasts.error("Couldn't load messages", errorMessage(e));
    }
  }

  /** Background prefetch (Performance mode only). */
  prefetch(id: string) {
    if (!this.convs[id]) void this.ensureLoaded(id);
  }

  async loadOlder(id: string): Promise<boolean> {
    const st = this.convs[id];
    if (!st || !st.hasMore || st.loadingOlder) return false;
    const first = st.messages[0];
    if (!first) return false;
    st.loadingOlder = true;
    try {
      const page = await api.messagesList(id, { createdAt: first.createdAt, id: first.id });
      const cap = app.profile?.messageWindow ?? 300;
      const merged = mergeWindow(st.messages, page.messages, cap, "older");
      st.messages = merged.messages;
      st.hasMore = page.hasMore;
      if (merged.trimmed) st.detached = true;
      return page.messages.length > 0;
    } catch (e) {
      toasts.error("Couldn't load older messages", errorMessage(e));
      return false;
    } finally {
      st.loadingOlder = false;
    }
  }

  async jumpToPresent(id: string) {
    const page = await api.messagesList(id);
    this.convs[id] = {
      messages: page.messages,
      hasMore: page.hasMore,
      loaded: true,
      loadingOlder: false,
      detached: false,
      newWhileDetached: 0,
    };
  }

  markRead(id: string) {
    const conv = this.conversations.find((c) => c.id === id);
    if (conv && (conv.unreadCount > 0 || conv.mentionCount > 0)) {
      conv.unreadCount = 0;
      conv.mentionCount = 0;
    }
    api.markRead(id).catch(() => {});
  }

  /** Returns true when the message is "new to the user" (not currently in view). */
  onMessageCreated(m: Message, viewing: boolean): boolean {
    const st = this.convs[m.conversationId];
    if (st?.loaded) {
      if (st.detached) {
        if (!m.author.isSelf) st.newWhileDetached += 1;
      } else {
        const cap = app.profile?.messageWindow ?? 300;
        const merged = mergeWindow(st.messages, [m], cap, "newer");
        st.messages = merged.messages;
        if (merged.trimmed) st.hasMore = true;
      }
    }
    // Clear the typing indicator for this author.
    const t = this.typing[m.conversationId];
    if (t?.some((x) => x.userId === m.author.id)) {
      this.typing[m.conversationId] = t.filter((x) => x.userId !== m.author.id);
    }
    if (m.author.isSelf) return false;
    const isActive = m.conversationId === this.activeConversationId;
    if (isActive && viewing) {
      this.markRead(m.conversationId);
      return false;
    }
    const conv = this.conversations.find((c) => c.id === m.conversationId);
    if (conv) {
      conv.unreadCount += 1;
      if (m.mentionsSelf) conv.mentionCount += 1;
    }
    return true;
  }

  onMessageDeleted(id: string, conversationId: string) {
    const st = this.convs[conversationId];
    if (st) st.messages = st.messages.filter((m) => m.id !== id);
  }

  onTyping(conversationId: string, userId: string, name: string, active: boolean) {
    const list = (this.typing[conversationId] ?? []).filter((t) => t.userId !== userId);
    this.typing[conversationId] = active ? [...list, { userId, name }] : list;
  }

  async send(conversationId: string, content: string): Promise<boolean> {
    try {
      const m = await api.messageSend(conversationId, content);
      // The backend also emits message-created; merging is idempotent by id.
      this.onMessageCreated(m, true);
      return true;
    } catch (e) {
      toasts.error("Message not sent", errorMessage(e));
      return false;
    }
  }

  async remove(messageId: string) {
    try {
      await api.messageDelete(messageId);
    } catch (e) {
      toasts.error("Couldn't delete message", errorMessage(e));
    }
  }

  addNote(conversationId: string, note: Omit<LocalNote, "id" | "createdAt">) {
    const list = this.notes[conversationId] ?? [];
    this.notes[conversationId] = [...list, { ...note, id: `note-${++noteSeq}`, createdAt: Date.now() }].slice(-5);
  }

  dismissNote(conversationId: string, id: string) {
    this.notes[conversationId] = (this.notes[conversationId] ?? []).filter((n) => n.id !== id);
  }

  async runSearch(query: string) {
    this.searchQuery = query;
    if (!query.trim()) {
      this.searchResults = [];
      return;
    }
    this.searching = true;
    try {
      const results = await api.search(query);
      if (this.searchQuery === query) this.searchResults = results;
    } catch (e) {
      toasts.error("Search failed", errorMessage(e));
    } finally {
      this.searching = false;
    }
  }

  /** Open a search hit: switch conversation and page back until the message is loaded. */
  async openHit(hit: SearchHit) {
    await this.selectConversation(hit.message.conversationId);
    const id = hit.message.conversationId;
    for (let i = 0; i < 12; i++) {
      const st = this.convs[id];
      if (!st || st.messages.some((m) => m.id === hit.message.id)) break;
      if (!(await this.loadOlder(id))) break;
    }
    this.highlightId = hit.message.id;
    setTimeout(() => {
      if (this.highlightId === hit.message.id) this.highlightId = null;
    }, 2400);
  }

  memberName(userId: string): string {
    for (const list of Object.values(this.members)) {
      const m = list.find((x) => x.user.id === userId);
      if (m) return m.user.displayName;
    }
    return "Someone";
  }
}

export const workspace = new Workspace();
