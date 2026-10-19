import type { Arguments, ManifestCommand } from "@flowkey-cli/native-sdk";
import { useEffect, useMemo, useRef, useState } from "react";
import { Footer, Loader, useActionHost } from "../ActionHost";
import { matchesShortcut } from "../actions";
import { Dropdown } from "../Dropdown";
import type { Action } from "../extension-api";
import { BackButton } from "../ListView";
import { preferencesOf } from "./extensions";
import { actionIcon, FlowKeyIcon } from "./FlowKeyIcon";
import { dispatchNative } from "./natives";
import { environmentFor } from "./protocol";
import { asProtocolError, type InstalledExtension } from "./runtime";

interface ViewState {
  primaryTitle?: string;
  canGoBack?: boolean;
  hasActions?: boolean;
  actions?: { id: string; title?: string; icon?: string }[];
  searchPlaceholder?: string;
  selectionTitle?: string;
  filters?: { value: string; label?: string }[];
}

const THEME_TOKENS: [string, string][] = [
  ["window-background", "--color-bg"],
  ["chrome-background", "--color-bg"],
  ["window-border", "--color-panel-border"],
  ["row-selected-background", "--color-list-selection"],
  ["divider", "--color-separator"],
  ["chrome-divider", "--color-separator"],
  ["text-primary", "--color-fg"],
  ["text-secondary", "--color-fg-60"],
  ["text-tertiary", "--color-fg-40"],
  ["keycap-background", "--color-key-bg"],
  ["keycap-border", "--color-key-border"],
  ["accent", "--color-accent"],
  ["surface", "--color-control"],
  ["surface-alt", "--color-card"],
  ["cell-background", "--color-list-hover"],
  ["chip-background", "--color-control"],
  ["action-panel-background", "--color-panel"],
  ["success", "--color-green"],
  ["danger", "--color-red"],
];

function hexOf(color: string, base: string): string {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = 1;
  const context = canvas.getContext("2d", { willReadFrequently: true })!;
  context.fillStyle = base;
  context.fillRect(0, 0, 1, 1);
  context.fillStyle = color;
  context.fillRect(0, 0, 1, 1);
  const [r, g, b] = context.getImageData(0, 0, 1, 1).data;
  return `#${[r, g, b].map((n) => n.toString(16).padStart(2, "0")).join("")}`;
}

function resolveVar(name: string): string {
  const probe = document.createElement("span");
  probe.style.color = `var(${name})`;
  probe.style.display = "none";
  document.body.appendChild(probe);
  const value = getComputedStyle(probe).color;
  probe.remove();
  return value;
}

function themeTokens(): Record<string, string> {
  const background = hexOf(resolveVar("--color-bg"), "#000000");
  return Object.fromEntries(THEME_TOKENS.map(([token, variable]) => [token, token === "window-background" || token === "chrome-background" ? background : hexOf(resolveVar(variable), background)]));
}

function themeCss(tokens: Record<string, string>): string {
  return `:root {\n${[...Object.entries(tokens), ["spotify-brand", "#1DB954"]].map(([k, v]) => `  --fk-${k}: ${v};`).join("\n")}\n}`;
}

const PAGE_KEYS: Record<string, string> = { ArrowDown: "moveDown", ArrowUp: "moveUp", PageDown: "pageDown", PageUp: "pageUp" };

export function WebCommandView({ extension, command, args, pop }: { extension: InstalledExtension; command: ManifestCommand; args?: Arguments; pop: () => void }) {
  const frameRef = useRef<HTMLIFrameElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [filterValue, setFilterValue] = useState<string>();
  const [view, setView] = useState<ViewState>({});
  const [loaded, setLoaded] = useState(false);
  const ready = useRef(false);
  const latest = useRef({ query, filterValue });
  latest.current = { query, filterValue };
  const host = useActionHost(() => inputRef.current?.focus());
  const src = `${extension.baseUrl}__host__/${encodeURIComponent(command.webEntry ?? "main.web.js")}`;

  const post = (message: Record<string, unknown>) => frameRef.current?.contentWindow?.postMessage(message, "*");
  const sendTheme = () => {
    const tokens = themeTokens();
    post({ type: "theme", css: themeCss(tokens) });
    return tokens;
  };
  const sendProps = async () =>
    post({
      type: "props",
      query: latest.current.query,
      filterValue: latest.current.filterValue,
      arguments: args,
      preferences: await preferencesOf(extension.manifest),
      environment: environmentFor(extension.manifest, false, command.id),
      theme: themeTokens(),
    });

  const groups = useMemo<Action[][]>(() => {
    const actions: Action[] = (view.actions ?? [])
      .filter((a) => a.id && a.title)
      .map((a) => ({ id: a.id, title: a.title!, icon: actionIcon(a.title!, a.icon) ?? undefined, keepOpen: true, run: () => post({ type: "paletteAction", action: a.id, actionId: a.id }) }));
    return actions.length ? [actions] : [];
  }, [view.actions]);
  const openPanel = () => groups.length && host.openPanel(view.selectionTitle ?? view.primaryTitle ?? command.title, groups);
  const openPanelRef = useRef(openPanel);
  openPanelRef.current = openPanel;

  useEffect(() => {
    const calls = new Map<string, AbortController>();
    const onMessage = async (event: MessageEvent) => {
      if (event.source !== frameRef.current?.contentWindow || !event.data || typeof event.data !== "object") return;
      const message = event.data as Record<string, unknown>;
      if (message.type === "ready") {
        ready.current = true;
        setLoaded(true);
        sendTheme();
        await sendProps();
      } else if (message.type === "viewState") {
        setView(message as ViewState);
      } else if (message.type === "openPalette") {
        openPanelRef.current();
      } else if (message.type === "focusHost") {
        inputRef.current?.focus();
      } else if (message.type === "log") {
        console[message.level === "error" ? "error" : "log"](`[${extension.manifest.id}]`, message.message);
      } else if (message.type === "webCall" && typeof message.bridgeId === "string" && typeof message.method === "string") {
        const bridgeId = message.bridgeId;
        const controller = new AbortController();
        calls.set(bridgeId, controller);
        const params = message.params && typeof message.params === "object" ? (message.params as Record<string, unknown>) : {};
        try {
          const result = await dispatchNative({ extension }, message.method, params, controller.signal);
          post({ type: "result", bridgeId, ok: true, result });
        } catch (error) {
          post({ type: "result", bridgeId, ok: false, error: asProtocolError(error) });
        } finally {
          calls.delete(bridgeId);
        }
      } else if (message.type === "webAbort" && typeof message.bridgeId === "string") {
        calls.get(message.bridgeId)?.abort();
      }
    };
    window.addEventListener("message", onMessage);
    const observer = new MutationObserver(() => ready.current && sendTheme());
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["style", "class", "data-theme"] });
    return () => {
      window.removeEventListener("message", onMessage);
      observer.disconnect();
      calls.forEach((controller) => controller.abort());
      post({ type: "dispose" });
    };
  }, [extension]);

  useEffect(() => {
    if (ready.current) void sendProps();
  }, [query, filterValue]);

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (host.panelOpen) return;
    if (matchesShortcut(event, { key: "k", ctrl: true })) {
      event.preventDefault();
      openPanel();
    } else if (event.key === "Enter" && !event.ctrlKey && !event.altKey && !event.shiftKey) {
      event.preventDefault();
      post({ type: "pageAction", action: "primary" });
    } else if (PAGE_KEYS[event.key] && !event.ctrlKey && !event.altKey) {
      event.preventDefault();
      post({ type: "pageAction", action: PAGE_KEYS[event.key] });
    } else if ((event.key === "ArrowLeft" || event.key === "ArrowRight") && !query && !event.ctrlKey && !event.altKey && !event.shiftKey) {
      event.preventDefault();
      post({ type: "hostKey", key: event.key });
    } else if (event.key === "Escape" || (event.key === "Backspace" && !query)) {
      event.preventDefault();
      if (event.key === "Escape" && query) setQuery("");
      else if (view.canGoBack) post({ type: "pageAction", action: "goBack" });
      else pop();
    }
  }

  const filters = view.filters ?? [];
  const placeholder = view.searchPlaceholder ?? (/^search\b/i.test(command.title) ? `${command.title}...` : `Search ${command.title}...`);
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <header className="relative flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <BackButton onBack={() => (view.canGoBack ? post({ type: "pageAction", action: "goBack" }) : pop())} />
        <input
          ref={inputRef}
          autoFocus
          spellCheck={false}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={onKeyDown}
          placeholder={placeholder}
          aria-label={placeholder.replace(/…$|\.\.\.$/, "")}
          className="h-full min-w-0 flex-1 bg-transparent text-18 text-fg caret-fg outline-none placeholder:text-fg-40"
        />
        {filters.length > 0 && (
          <>
            <span id="web-filter" className="sr-only">
              Filter
            </span>
            <Dropdown value={filterValue ?? filters[0].value} options={filters.map((f) => ({ value: f.value, label: f.label ?? f.value }))} onChange={setFilterValue} labelledBy="web-filter" size="small" />
          </>
        )}
        <Loader active={!loaded || host.busy} />
      </header>
      <iframe ref={frameRef} title={command.title} src={src} sandbox="allow-scripts" referrerPolicy="no-referrer" className="min-h-0 w-full flex-1 border-0 bg-transparent" />
      <Footer
        left={
          <span className="flex min-w-0 items-center gap-2 text-fg-60">
            <FlowKeyIcon fields={{ icon: command.icon ?? extension.manifest.icon, iconColor: command.iconColor }} extension={extension} size="small" title={command.title} />
            <span className="truncate">{command.title}</span>
          </span>
        }
        primaryTitle={view.primaryTitle || undefined}
        panelOpen={host.panelOpen}
        showActions={groups.length > 0}
        onTogglePanel={() => (host.panelOpen ? host.closePanel() : openPanel())}
      />
      {host.render((action) => host.run(action, { failTitle: `${action.title} failed` }))}
    </div>
  );
}
