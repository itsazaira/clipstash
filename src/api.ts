import { invoke } from "@tauri-apps/api/core";

export type Item = { id: number; text: string; copiedAt: number; pinned: boolean };
export type Snapshot = { items: Item[]; paused: boolean; maxItems: number };

export const HISTORY_SIZES = [10, 20, 50, 100];

export const isMac = navigator.userAgent.includes("Mac");
export const shortcutLabel = isMac ? "⌘ ⇧ V" : "Ctrl Shift V";

export const api = {
  getState: () => invoke<Snapshot>("get_state"),
  paste: (id: number) => invoke<"pasted" | "copied" | "needs-permission">("paste_item", { id }),
  togglePin: (id: number) => invoke<void>("toggle_pin", { id }),
  setPaused: (paused: boolean) => invoke<void>("set_paused", { paused }),
  clearAll: () => invoke<void>("clear_all"),
  setMaxItems: (max: number) => invoke<void>("set_max_items", { max }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enabled: boolean) => invoke<void>("set_autostart", { enabled }),
  hidePopup: () => invoke<void>("hide_popup"),
  openLink: (url: string) => invoke<void>("open_link", { url }),
  hasAccessibility: () => invoke<boolean>("has_accessibility"),
  requestAccessibility: () => invoke<void>("request_accessibility"),
};

/** "just now", "5m ago", "3h ago", "Yesterday", or a date. */
export function timeAgo(ms: number, now = Date.now()): string {
  const s = Math.max(0, Math.round((now - ms) / 1000));
  if (s < 45) return "just now";
  const m = Math.round(s / 60);
  if (m < 60) return `${m}m ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h}h ago`;
  if (h < 48) return "Yesterday";
  return new Date(ms).toLocaleDateString(undefined, { day: "numeric", month: "short" });
}

/** The first two non-empty lines of a copy. */
export function previewLines(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((l) => l.replace(/\t/g, "  ").trimEnd())
    .filter((l) => l.trim() !== "")
    .slice(0, 2);
}
