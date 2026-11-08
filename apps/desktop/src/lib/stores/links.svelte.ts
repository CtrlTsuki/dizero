import { api, errorMessage } from "../api";
import { toasts } from "./toasts.svelte";

/** A link waiting for the user to confirm before it opens in the browser. */
class PendingLink {
  url = $state<string | null>(null);
}

export const pendingLink = new PendingLink();

export function confirmLink(url: string) {
  pendingLink.url = url;
}

export async function openPendingLink() {
  const url = pendingLink.url;
  pendingLink.url = null;
  if (!url) return;
  try {
    await api.openLink(url);
  } catch (e) {
    toasts.error("Couldn't open the link", errorMessage(e));
  }
}
