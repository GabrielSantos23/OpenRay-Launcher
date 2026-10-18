import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { ActionPanel, Keys } from "./ActionPanel";
import type { Action, Confirmation } from "./extension-api";
import { FooterToast } from "./FooterToast";
import { showToast, useToast } from "./toast";

export function Logo() {
  return (
    <svg viewBox="0 0 24 24" className="size-[22px] shrink-0 text-fg-60" aria-hidden="true">
      <circle cx="12" cy="12" r="8" fill="none" stroke="currentColor" strokeWidth="2.5" />
      <path d="M12 12 L21 3" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" />
    </svg>
  );
}

export function Loader({ active }: { active: boolean }) {
  const [shown, setShown] = useState(false);
  useEffect(() => {
    if (!active) {
      setShown(false);
      return;
    }
    const timer = window.setTimeout(() => setShown(true), 300);
    return () => window.clearTimeout(timer);
  }, [active]);
  return (
    <div
      aria-hidden="true"
      className={`loader-gradient pointer-events-none absolute inset-x-0 -bottom-px h-px animate-loader transition-opacity duration-300 ${shown ? "opacity-100" : "opacity-0 [animation-play-state:paused]"}`}
    />
  );
}

export function fail(title: string, error: unknown) {
  showToast({ variant: "failure", title, message: error instanceof Error ? error.message : String(error) });
}

function ConfirmDialog({ confirmation, onAnswer }: { confirmation: Confirmation; onAnswer: (ok: boolean) => void }) {
  const cancelRef = useRef<HTMLButtonElement>(null);
  useEffect(() => cancelRef.current?.focus(), []);
  return (
    <div className="fixed inset-0 z-30 flex items-center justify-center bg-black/40" onMouseDown={(e) => e.target === e.currentTarget && onAnswer(false)}>
      <div
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="confirm-title"
        aria-describedby="confirm-message"
        onKeyDown={(e) => {
          if (e.key === "Escape") {
            e.preventDefault();
            e.stopPropagation();
            onAnswer(false);
          } else if (e.key === "Tab") {
            e.preventDefault();
            const buttons = [...e.currentTarget.querySelectorAll("button")];
            buttons[(buttons.indexOf(document.activeElement as HTMLButtonElement) + 1) % buttons.length]?.focus();
          }
        }}
        className="flex w-[300px] flex-col items-center gap-4 rounded-panel border border-panel-border bg-panel p-4 text-center text-13 shadow-panel"
      >
        <div className="flex flex-col gap-1">
          <h2 id="confirm-title" className="font-semibold text-fg">
            {confirmation.title}
          </h2>
          <p id="confirm-message" className="text-fg-60">
            {confirmation.message}
          </p>
        </div>
        <div className="flex w-full flex-col gap-2">
          <button
            type="button"
            onClick={() => onAnswer(true)}
            className="h-8 rounded-[6px] bg-control font-medium text-destructive outline-offset-2 hover:bg-control-hover focus-visible:outline-2 focus-visible:outline-accent"
          >
            {confirmation.confirmTitle}
          </button>
          <button
            ref={cancelRef}
            type="button"
            onClick={() => onAnswer(false)}
            className="h-8 rounded-[6px] bg-control font-medium text-fg outline-offset-2 hover:bg-control-hover focus-visible:outline-2 focus-visible:outline-accent"
          >
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}

interface RunOptions {
  failTitle: string;
  onSuccess?: () => void;
}

export function useActionHost(focusInput: () => void) {
  const [panel, setPanel] = useState<{ title: string; groups: Action[][] } | null>(null);
  const [confirming, setConfirming] = useState<{ confirmation: Confirmation; resolve: (ok: boolean) => void } | null>(null);
  const [busy, setBusy] = useState(0);

  function closePanel() {
    setPanel(null);
    focusInput();
  }

  function ask(confirmation: Confirmation) {
    return new Promise<boolean>((resolve) => setConfirming({ confirmation, resolve }));
  }

  async function run(action: Action, { failTitle, onSuccess }: RunOptions) {
    setPanel(null);
    focusInput();
    if (action.submenu) {
      try {
        setPanel({ title: action.title.replace(/…$/, ""), groups: [await action.submenu()] });
      } catch (e) {
        fail(`${action.title.replace(/…$/, "")} failed`, e);
      }
      return;
    }
    if (action.confirm && !(await ask(action.confirm))) return;
    setBusy((n) => n + 1);
    try {
      await action.run();
    } catch (e) {
      fail(failTitle, e);
      return;
    } finally {
      setBusy((n) => n - 1);
    }
    onSuccess?.();
    if (!action.keepOpen) await invoke("hide_launcher", { restoreFocus: true });
  }

  function render(onRun: (action: Action) => void) {
    return (
      <>
        {panel && <ActionPanel key={panel.title} title={panel.title} groups={panel.groups} onRun={onRun} onClose={closePanel} />}
        {confirming && (
          <ConfirmDialog
            confirmation={confirming.confirmation}
            onAnswer={(ok) => {
              setConfirming(null);
              focusInput();
              confirming.resolve(ok);
            }}
          />
        )}
      </>
    );
  }

  return { panelOpen: !!panel, openPanel: (title: string, groups: Action[][]) => setPanel({ title, groups }), closePanel, run, render, busy: busy > 0 };
}

export function Footer({
  left,
  primaryTitle,
  primaryKeys = ["↵"],
  panelOpen,
  onTogglePanel,
  showActions = true,
}: {
  left?: ReactNode;
  primaryTitle?: string;
  primaryKeys?: string[];
  panelOpen: boolean;
  onTogglePanel: () => void;
  showActions?: boolean;
}) {
  const toast = useToast();
  return (
    <footer className="relative isolate flex h-footer shrink-0 items-center gap-2 overflow-hidden pt-px pr-[9px] pb-px pl-3 text-13 before:absolute before:inset-x-0 before:top-0 before:-z-20 before:h-px before:bg-separator">
      {toast ? <FooterToast /> : left}
      {primaryTitle && (
        <span className="ml-auto flex shrink-0 items-center gap-2 font-medium">
          {primaryTitle}
          <Keys labels={primaryKeys} />
        </span>
      )}
      {primaryTitle && showActions && (
        <>
          <span aria-hidden="true" className={`-mr-0.5 ml-1 h-[11px] w-px ${panelOpen ? "" : "bg-footer-divider"}`} />
          <button
            type="button"
            tabIndex={-1}
            aria-expanded={panelOpen}
            onMouseDown={(e) => e.preventDefault()}
            onClick={onTogglePanel}
            className={`flex h-[26px] shrink-0 cursor-default items-center gap-2 rounded-[6px] pr-[3px] pl-2 font-medium text-fg-60 ${panelOpen ? "bg-list-selection" : ""}`}
          >
            Actions
            <Keys labels={["Ctrl", "K"]} />
          </button>
        </>
      )}
    </footer>
  );
}
