import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, HISTORY_SIZES, isMac, Item, previewLines, shortcutLabel, Snapshot, timeAgo } from "./api";
import About from "./About";
import Logo from "./Logo";
import { applyTheme, loadTheme, Theme, THEMES } from "./theme";

type View = "list" | "permission" | "about";

export default function Popup() {
  const [snapshot, setSnapshot] = useState<Snapshot>({ items: [], paused: false, maxItems: 50 });
  const [toast, setToast] = useState("");
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const [view, setView] = useState<View>("list");
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [confirmClear, setConfirmClear] = useState(false);
  const [now, setNow] = useState(Date.now());
  const searchRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  useEffect(() => {
    api.getState().then(setSnapshot);
    const unState = listen<Snapshot>("state-changed", (e) => setSnapshot(e.payload));
    // Every time the popup opens: fresh search, first item selected, cursor in the search box.
    const unShown = listen("popup-shown", () => {
      setQuery("");
      setSelected(0);
      setView("list");
      setSettingsOpen(false);
      setConfirmClear(false);
      setToast("");
      setNow(Date.now());
      requestAnimationFrame(() => searchRef.current?.focus());
    });
    return () => {
      unState.then((f) => f());
      unShown.then((f) => f());
    };
  }, []);

  const items = useMemo(() => {
    const q = query.trim().toLowerCase();
    return q ? snapshot.items.filter((i) => i.text.toLowerCase().includes(q)) : snapshot.items;
  }, [snapshot.items, query]);

  useEffect(() => setSelected(0), [query]);

  useEffect(() => {
    listRef.current?.querySelector<HTMLElement>(`[data-index="${selected}"]`)?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  useEffect(() => setToast(""), [view]);

  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(""), 2500);
    return () => clearTimeout(timer);
  }, [toast]);

  const pick = useCallback(async (item: Item) => {
    const result = await api.paste(item.id);
    if (result === "needs-permission") setView("permission");
    // Opened from the tray or taskbar: there is no app to paste into, so it is copied instead.
    if (result === "copied") setToast(`Copied. Press ${isMac ? "⌘ V" : "Ctrl V"} to paste it anywhere.`);
  }, []);

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (view !== "list") {
      if (e.key === "Escape") setView("list");
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((s) => Math.min(s + 1, items.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((s) => Math.max(s - 1, 0));
    } else if (e.key === "Enter" && items[selected]) {
      e.preventDefault();
      pick(items[selected]);
    } else if (e.key === "Escape") {
      e.preventDefault();
      if (settingsOpen) setSettingsOpen(false);
      else if (query) setQuery("");
      else api.hidePopup();
    }
  };

  return (
    <div className="popup" onKeyDown={onKeyDown}>
      {/* Press and drag anywhere on the header, except its buttons, to move the window. */}
      <header
        className="popup-header"
        onMouseDown={(e) => {
          if (e.button === 0 && !(e.target as Element).closest("button")) {
            e.preventDefault();
            getCurrentWindow().startDragging();
          }
        }}
      >
        <div className="brand">
          <Logo size={26} />
          <div>
            <div className="brand-name">ClipStash</div>
            <div className="brand-by">by Its Azaira</div>
          </div>
        </div>
        <div className="header-actions">
          {/* Opens and closes the settings pane above the list. */}
          <button
            className={`icon-btn ${settingsOpen && view === "list" ? "active" : ""}`}
            title={settingsOpen ? "Hide settings" : "Settings"}
            aria-label="Settings"
            aria-expanded={settingsOpen && view === "list"}
            onClick={() => {
              setSettingsOpen(view !== "list" || !settingsOpen);
              setView("list");
            }}
          >
            <GearIcon />
          </button>
          <button className="icon-btn" title="Close" aria-label="Close ClipStash" onClick={() => api.hidePopup()}>
            <CloseIcon />
          </button>
        </div>
      </header>

      {view === "list" && (
        <>
          {settingsOpen && (
            <Settings
              maxItems={snapshot.maxItems}
              onMaxItems={async (max) => {
                // Show the choice straight away; if saving fails, fall back to what was saved.
                setSnapshot((s) => ({ ...s, maxItems: max }));
                try {
                  await api.setMaxItems(max);
                } catch {
                  setSnapshot(await api.getState());
                }
              }}
              onHide={() => setSettingsOpen(false)}
            />
          )}

          <div className="search">
            <SearchIcon />
            <input
              ref={searchRef}
              autoFocus
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder="Search your copies"
              spellCheck={false}
              aria-label="Search"
            />
          </div>

          {snapshot.paused && <div className="banner">Recording is paused. New copies are not being saved.</div>}

          <ul className="list" ref={listRef} role="listbox" aria-label="Saved copies">
            {items.map((item, index) => (
              <li
                key={item.id}
                data-index={index}
                role="option"
                aria-selected={index === selected}
                className={`item ${index === selected ? "selected" : ""}`}
                onMouseMove={() => setSelected(index)}
                onClick={() => pick(item)}
              >
                <div className="item-text">
                  {previewLines(item.text).map((line, n) => (
                    <div key={n} className="line">
                      {line}
                    </div>
                  ))}
                </div>
                <div className="item-meta">
                  <span className="time">{timeAgo(item.copiedAt, now)}</span>
                  <button
                    className={`pin ${item.pinned ? "pinned" : ""}`}
                    title={item.pinned ? "Unpin" : "Pin"}
                    aria-label={item.pinned ? "Unpin" : "Pin"}
                    onClick={(e) => {
                      e.stopPropagation();
                      api.togglePin(item.id);
                    }}
                  >
                    <PinIcon filled={item.pinned} />
                  </button>
                </div>
              </li>
            ))}
            {items.length === 0 && (
              <li className="empty">
                {query ? (
                  "No copies match your search."
                ) : (
                  <>
                    Copy some text and it shows up here.
                    <br />
                    Press <kbd>{shortcutLabel}</kbd> any time to open ClipStash.
                  </>
                )}
              </li>
            )}
          </ul>

          <footer className="popup-footer">
            <label className="toggle">
              <input type="checkbox" checked={snapshot.paused} onChange={(e) => api.setPaused(e.target.checked)} />
              <span className="switch" />
              Pause
            </label>
            {confirmClear ? (
              <span className="confirm">
                Clear unpinned?
                <button className="text-btn danger" onClick={() => { api.clearAll(); setConfirmClear(false); }}>
                  Clear
                </button>
                <button className="text-btn" onClick={() => setConfirmClear(false)}>
                  Cancel
                </button>
              </span>
            ) : (
              <button className="text-btn" disabled={snapshot.items.every((i) => i.pinned)} onClick={() => setConfirmClear(true)}>
                Clear all
              </button>
            )}
          </footer>
        </>
      )}

      {view === "permission" && <Permission onDone={() => setView("list")} />}
      {view === "about" && <About onBack={() => setView("list")} />}

      {view !== "about" && (
        <div className="support-bar">
          <button className="support-btn" onClick={() => setView("about")}>
            <HeartIcon /> Support us
          </button>
        </div>
      )}

      {toast && (
        <div className="toast" role="status">
          {toast}
        </div>
      )}
    </div>
  );
}

/** Settings pane at the top of the popup, opened and closed with the gear button. */
function Settings({
  maxItems,
  onMaxItems,
  onHide,
}: {
  maxItems: number;
  onMaxItems: (max: number) => void;
  onHide: () => void;
}) {
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [theme, setTheme] = useState<Theme>(loadTheme);

  const changeTheme = (next: Theme) => {
    setTheme(next);
    applyTheme(next);
  };

  useEffect(() => {
    api.getAutostart().then(setAutostart);
  }, []);

  const change = async (enabled: boolean) => {
    setAutostart(enabled);
    try {
      await api.setAutostart(enabled);
    } catch {
      setAutostart(await api.getAutostart());
    }
  };

  return (
    <div className="settings-pane">
      <label className="toggle row">
        <span>
          Start when my computer starts
          <small>ClipStash opens quietly in the {isMac ? "menu bar" : "system tray"}.</small>
        </span>
        <input type="checkbox" checked={!!autostart} disabled={autostart === null} onChange={(e) => change(e.target.checked)} />
        <span className="switch" />
      </label>
      <div className="row static">
        <span>Appearance</span>
        <div className="segmented" role="radiogroup" aria-label="Appearance">
          {THEMES.map((t) => (
            <button
              key={t.value}
              role="radio"
              aria-checked={t.value === theme}
              className={t.value === theme ? "on" : ""}
              onClick={() => changeTheme(t.value)}
            >
              {t.label}
            </button>
          ))}
        </div>
      </div>
      <div className="row static">
        <span>
          Keep my last
          <small>Older copies are removed. Pinned items are always kept.</small>
        </span>
        <div className="segmented" role="radiogroup" aria-label="How many copies to keep">
          {HISTORY_SIZES.map((size) => (
            <button
              key={size}
              role="radio"
              aria-checked={size === maxItems}
              className={size === maxItems ? "on" : ""}
              onClick={() => onMaxItems(size)}
            >
              {size}
            </button>
          ))}
        </div>
      </div>
      <div className="row static">
        <span>
          Shortcut
          <small>Opens and closes ClipStash from any app.</small>
        </span>
        <kbd>{shortcutLabel}</kbd>
      </div>
      <button className="fold-hint" onClick={onHide}>
        <ChevronUpIcon /> Hide settings. Open them again with the <GearIcon size={13} /> button.
      </button>
    </div>
  );
}

function CloseIcon() {
  return (
    <svg width="17" height="17" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
      <path d="M6 6l12 12M18 6 6 18" strokeLinecap="round" />
    </svg>
  );
}

function ChevronUpIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
      <path d="m18 15-6-6-6 6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

/** macOS only: shown when auto paste needs Accessibility access. */
function Permission({ onDone }: { onDone: () => void }) {
  const [checking, setChecking] = useState(false);
  const [stillMissing, setStillMissing] = useState(false);

  const recheck = async () => {
    setChecking(true);
    const ok = await api.hasAccessibility();
    setChecking(false);
    if (ok) onDone();
    else setStillMissing(true);
  };

  return (
    <div className="panel permission">
      <div className="permission-icon">
        <Logo size={44} />
      </div>
      <h2>One quick step for auto paste</h2>
      <p>
        Your item is copied, so you can press <kbd>⌘ V</kbd> right now. To have ClipStash paste for you, macOS needs you to
        allow it once:
      </p>
      <ol>
        <li>Click <strong>Open Settings</strong> below.</li>
        <li>In <strong>Privacy &amp; Security → Accessibility</strong>, turn on <strong>ClipStash</strong>.</li>
        <li>If it does not work straight away, quit ClipStash from the menu bar and open it again.</li>
      </ol>
      <p className="muted">ClipStash only uses this to press ⌘ V for you. Nothing ever leaves your Mac.</p>
      {stillMissing && <p className="warn">Not allowed yet. Turn on ClipStash in the list, then try again.</p>}
      <div className="panel-actions">
        <button className="text-btn" onClick={onDone}>
          Not now
        </button>
        <button className="text-btn" disabled={checking} onClick={recheck}>
          I've allowed it
        </button>
        <button className="btn" onClick={() => api.requestAccessibility()}>
          Open Settings
        </button>
      </div>
    </div>
  );
}

function SearchIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
      <circle cx="11" cy="11" r="7" />
      <path d="m20 20-3.5-3.5" strokeLinecap="round" />
    </svg>
  );
}

function GearIcon({ size = 18 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true">
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1Z" />
    </svg>
  );
}

function HeartIcon() {
  return (
    <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M12 21s-7.5-4.6-9.6-9.4C.9 8.2 3 4.5 6.6 4.5c2.1 0 3.6 1.2 5.4 3.2 1.8-2 3.3-3.2 5.4-3.2 3.6 0 5.7 3.7 4.2 7.1C19.5 16.4 12 21 12 21Z" />
    </svg>
  );
}

function PinIcon({ filled }: { filled: boolean }) {
  return (
    <svg width="15" height="15" viewBox="0 0 24 24" fill={filled ? "currentColor" : "none"} stroke="currentColor" strokeWidth="2" aria-hidden="true">
      <path d="M12 17v5M9 3h6l-1 6 4 4H6l4-4-1-6Z" strokeLinejoin="round" />
    </svg>
  );
}
